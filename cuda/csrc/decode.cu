#include <torch/extension.h>
#include <ATen/cuda/CUDAContext.h>
#include <c10/cuda/CUDAGuard.h>
#include <c10/cuda/CUDAException.h>

#include <algorithm>
#include <cstdint>
#include <vector>

namespace {

constexpr int kBlockSymbols = 128;
constexpr int64_t kHeaderBytes = 32;
constexpr int64_t kDirectoryEntryBytes = 32;
constexpr int64_t kShardSymbols = 4096;
constexpr uint8_t kEntropyParallelShards = 19;
constexpr uint8_t kEntropyOrder0 = 19;
constexpr int kRansAlphabet = 511;
constexpr int kRansTableLog = 12;
constexpr uint32_t kRansByteL = 1u << 23;
constexpr uint8_t kPredictFullTileClampGradient = 6;

__device__ uint16_t read_u16(const uint8_t* bytes) {
  return static_cast<uint16_t>(bytes[0]) |
      (static_cast<uint16_t>(bytes[1]) << 8);
}

__device__ uint32_t read_u32(const uint8_t* bytes) {
  return static_cast<uint32_t>(bytes[0]) |
      (static_cast<uint32_t>(bytes[1]) << 8) |
      (static_cast<uint32_t>(bytes[2]) << 16) |
      (static_cast<uint32_t>(bytes[3]) << 24);
}

__device__ uint64_t read_u64(const uint8_t* bytes) {
  return static_cast<uint64_t>(read_u32(bytes)) |
      (static_cast<uint64_t>(read_u32(bytes + 4)) << 32);
}

__device__ bool read_bits(
    const uint8_t* bytes,
    int64_t byte_count,
    int64_t* bit_position,
    int count,
    uint32_t* value) {
  if (count == 0) {
    *value = 0;
    return true;
  }
  if (*bit_position < 0 || *bit_position + count > byte_count * 8) {
    return false;
  }
  uint32_t result = 0;
  for (int bit = 0; bit < count; ++bit) {
    const int64_t position = *bit_position + bit;
    result |= static_cast<uint32_t>((bytes[position >> 3] >> (position & 7)) & 1) << bit;
  }
  *bit_position += count;
  *value = result;
  return true;
}

struct RiceBitReader {
  const uint8_t* bytes;
  int64_t byte_count;
  int64_t byte_position = 0;
  int64_t bit_position = 0;
  uint64_t buffer = 0;
  int buffered_bits = 0;

  __device__ void refill() {
    while (buffered_bits <= 56 && byte_position < byte_count) {
      buffer |= static_cast<uint64_t>(bytes[byte_position++]) << buffered_bits;
      buffered_bits += 8;
    }
  }

  __device__ bool read_rice(int parameter, uint32_t max_folded, uint32_t* value) {
    uint32_t quotient = 0;
    while (true) {
      if (buffered_bits == 0) {
        refill();
        if (buffered_bits == 0) {
          return false;
        }
      }
      if (buffer == 0) {
        quotient += buffered_bits;
        bit_position += buffered_bits;
        buffered_bits = 0;
        if (quotient > max_folded) {
          return false;
        }
        continue;
      }
      const int zeros = __ffsll(static_cast<long long>(buffer)) - 1;
      quotient += zeros;
      const int consumed = zeros + 1;
      buffer = consumed == 64 ? 0 : buffer >> consumed;
      buffered_bits -= consumed;
      bit_position += consumed;
      if (quotient > max_folded) {
        return false;
      }
      break;
    }
    if (buffered_bits < parameter) {
      refill();
    }
    if (buffered_bits < parameter) {
      return false;
    }
    const uint32_t mask = parameter == 0 ? 0 : (uint32_t{1} << parameter) - 1;
    const uint32_t remainder = static_cast<uint32_t>(buffer) & mask;
    buffer >>= parameter;
    buffered_bits -= parameter;
    bit_position += parameter;
    const uint64_t result = (static_cast<uint64_t>(quotient) << parameter) + remainder;
    if (result > max_folded) {
      return false;
    }
    *value = static_cast<uint32_t>(result);
    return true;
  }
};

__device__ void set_error(int32_t* status, int32_t code) {
  atomicCAS(status, 0, code);
}

__global__ void parse_metadata_kernel(
    const uint8_t* encoded,
    int64_t encoded_size,
    const int64_t* tile_metadata,
    const int64_t* tile_parse_metadata,
    int64_t tile_count,
    int64_t* shard_metadata,
    int32_t* status) {
  const int64_t tile = blockIdx.x;
  if (tile >= tile_count || threadIdx.x != 0) {
    return;
  }
  const int64_t directory_end = kHeaderBytes + tile_count * kDirectoryEntryBytes;
  const int64_t entry_offset = kHeaderBytes + tile * kDirectoryEntryBytes;
  const uint8_t* entry = encoded + entry_offset;
  const int64_t expected_plane = tile_parse_metadata[tile * 2 + 1];
  if (entry[0] != expected_plane || entry[1] != kEntropyParallelShards ||
      entry[2] != kPredictFullTileClampGradient || entry[3] != 0 ||
      read_u32(entry + 4) != tile_metadata[tile * 7 + 2] ||
      read_u32(entry + 8) != tile_metadata[tile * 7 + 3] ||
      read_u32(entry + 12) != tile_metadata[tile * 7 + 4] ||
      read_u32(entry + 16) != tile_metadata[tile * 7 + 5]) {
    set_error(status, 10);
    return;
  }
  const uint64_t payload_offset_u64 = read_u64(entry + 20);
  if (payload_offset_u64 > static_cast<uint64_t>(encoded_size)) {
    set_error(status, 10);
    return;
  }
  const int64_t payload_offset = static_cast<int64_t>(payload_offset_u64);
  const int64_t payload_length = read_u32(entry + 28);
  if (payload_length > encoded_size - payload_offset) {
    set_error(status, 10);
    return;
  }
  if (tile == 0) {
    if (payload_offset != directory_end) {
      set_error(status, 10);
      return;
    }
  } else {
    const uint8_t* previous = entry - kDirectoryEntryBytes;
    const uint64_t previous_offset = read_u64(previous + 20);
    const uint64_t previous_length = read_u32(previous + 28);
    if (previous_offset > static_cast<uint64_t>(encoded_size) ||
        previous_length > static_cast<uint64_t>(encoded_size) - previous_offset ||
        payload_offset_u64 != previous_offset + previous_length) {
      set_error(status, 10);
      return;
    }
  }
  if (tile + 1 == tile_count && payload_offset + payload_length != encoded_size) {
    set_error(status, 10);
    return;
  }

  const int64_t samples = tile_metadata[tile * 7 + 4] * tile_metadata[tile * 7 + 5];
  const int64_t folded_base = tile_metadata[tile * 7 + 6];
  const int64_t first_shard = tile_parse_metadata[tile * 2 + 0];
  const int64_t shard_count = (samples + kShardSymbols - 1) / kShardSymbols;
  int64_t cursor = payload_offset;
  int64_t decoded = 0;
  const int64_t tile_end = payload_offset + payload_length;
  for (int64_t local_shard = 0; local_shard < shard_count; ++local_shard) {
    if (cursor > tile_end - 3) {
      set_error(status, 11);
      return;
    }
    const int64_t mode = encoded[cursor];
    const int64_t body_length = read_u16(encoded + cursor + 1);
    const int64_t body_offset = cursor + 3;
    if (mode > 19 || body_length > tile_end - body_offset) {
      set_error(status, 11);
      return;
    }
    const int64_t sample_count = min(kShardSymbols, samples - decoded);
    const int64_t shard = first_shard + local_shard;
    shard_metadata[shard * 5 + 0] = mode;
    shard_metadata[shard * 5 + 1] = body_offset;
    shard_metadata[shard * 5 + 2] = body_length;
    shard_metadata[shard * 5 + 3] = sample_count;
    shard_metadata[shard * 5 + 4] = folded_base + decoded;
    decoded += sample_count;
    cursor = body_offset + body_length;
  }
  if (decoded != samples || cursor != tile_end) {
    set_error(status, 11);
  }
}

__device__ void decode_legacy_shard(
    const uint8_t* encoded,
    const int64_t* metadata,
    int64_t shard,
    uint32_t max_folded,
    uint32_t* folded,
    int32_t* status) {
  const int64_t mode = metadata[shard * 5 + 0];
  const int64_t body_offset = metadata[shard * 5 + 1];
  const int64_t body_length = metadata[shard * 5 + 2];
  const int64_t sample_count = metadata[shard * 5 + 3];
  const int64_t output_offset = metadata[shard * 5 + 4];
  const uint8_t* body = encoded + body_offset;
  uint32_t* output = folded + output_offset;

  if (mode == 0) {
    if (threadIdx.x != 0) {
      return;
    }
    int64_t cursor = 0;
    int64_t produced = 0;
    while (produced < sample_count) {
      uint32_t token = 0;
      int shift = 0;
      int bytes_used = 0;
      while (true) {
        if (cursor >= body_length || bytes_used == 5) {
          set_error(status, 1);
          return;
        }
        const uint8_t byte = body[cursor++];
        if (bytes_used == 4 && byte > 0x0f) {
          set_error(status, 1);
          return;
        }
        token |= static_cast<uint32_t>(byte & 0x7f) << shift;
        ++bytes_used;
        if ((byte & 0x80) == 0) {
          break;
        }
        shift += 7;
      }
      const int canonical_bytes = token < (1u << 7) ? 1
          : token < (1u << 14) ? 2
          : token < (1u << 21) ? 3
          : token < (1u << 28) ? 4
          : 5;
      if (bytes_used != canonical_bytes) {
        set_error(status, 1);
        return;
      }
      if ((token & 1) == 0) {
        const int64_t run = token / 2 + 1;
        if (run > sample_count - produced) {
          set_error(status, 1);
          return;
        }
        for (int64_t i = 0; i < run; ++i) {
          output[produced++] = 0;
        }
      } else {
        const uint32_t value = (token + 1) / 2;
        if (value == 0 || value > max_folded) {
          set_error(status, 1);
          return;
        }
        output[produced++] = value;
      }
    }
    if (cursor != body_length) {
      set_error(status, 1);
    }
    return;
  }

  if (mode >= 1 && mode <= 17) {
    const int lane_count = static_cast<int>(sample_count < 4 ? sample_count : 4);
    const int lane = threadIdx.x;
    if (lane >= lane_count) {
      return;
    }
    const int64_t table_bytes = (lane_count - 1) * 4;
    if (table_bytes > body_length) {
      set_error(status, 2);
      return;
    }
    int64_t lane_offset = table_bytes;
    for (int i = 0; i < lane; ++i) {
      lane_offset += read_u32(body + i * 4);
    }
    const int64_t lane_length = lane + 1 < lane_count
        ? read_u32(body + lane * 4)
        : body_length - lane_offset;
    if (lane_offset > body_length || lane_length < 0 || lane_length > body_length - lane_offset) {
      set_error(status, 2);
      return;
    }
    RiceBitReader reader{body + lane_offset, lane_length};
    for (int64_t index = lane; index < sample_count; index += lane_count) {
      uint32_t value = 0;
      if (!reader.read_rice(static_cast<int>(mode - 1), max_folded, &value)) {
        set_error(status, 2);
        return;
      }
      output[index] = value;
    }
    const int64_t used_bytes = (reader.bit_position + 7) / 8;
    if (used_bytes != lane_length) {
      set_error(status, 2);
      return;
    }
    if ((reader.bit_position & 7) != 0) {
      const uint8_t mask = static_cast<uint8_t>(~((1u << (reader.bit_position & 7)) - 1));
      if ((body[lane_offset + used_bytes - 1] & mask) != 0) {
        set_error(status, 2);
      }
    }
    return;
  }

  if (mode == 18) {
    const int64_t block_count = (sample_count + kBlockSymbols - 1) / kBlockSymbols;
    const int64_t block = threadIdx.x;
    if (block >= block_count) {
      return;
    }
    int64_t cursor = 0;
    for (int64_t prior = 0; prior < block; ++prior) {
      if (cursor >= body_length) {
        set_error(status, 3);
        return;
      }
      const int width = body[cursor++];
      const int64_t prior_remaining = sample_count - prior * kBlockSymbols;
      const int64_t prior_count = prior_remaining < kBlockSymbols ? prior_remaining : kBlockSymbols;
      cursor += (prior_count * width + 7) / 8;
    }
    if (cursor >= body_length) {
      set_error(status, 3);
      return;
    }
    const int bit_width = body[cursor++];
    const int max_width = 32 - __clz(max_folded);
    if (bit_width > max_width) {
      set_error(status, 3);
      return;
    }
    const int64_t remaining = sample_count - block * kBlockSymbols;
    const int64_t count = remaining < kBlockSymbols ? remaining : kBlockSymbols;
    const int64_t payload_bytes = (count * bit_width + 7) / 8;
    if (payload_bytes > body_length - cursor) {
      set_error(status, 3);
      return;
    }
    int64_t bit_position = 0;
    for (int64_t i = 0; i < count; ++i) {
      uint32_t value = 0;
      if (!read_bits(body + cursor, payload_bytes, &bit_position, bit_width, &value) ||
          value > max_folded) {
        set_error(status, 3);
        return;
      }
      output[block * kBlockSymbols + i] = value;
    }
    if (block + 1 == block_count && cursor + payload_bytes != body_length) {
      set_error(status, 3);
    }
    if ((bit_position & 7) != 0) {
      const uint8_t mask = static_cast<uint8_t>(~((1u << (bit_position & 7)) - 1));
      if ((body[cursor + payload_bytes - 1] & mask) != 0) {
        set_error(status, 3);
      }
    }
    return;
  }

  if (mode != kEntropyOrder0) set_error(status, 4);
}

__device__ int32_t unzigzag(uint32_t value) {
  return (value & 1) == 0
      ? static_cast<int32_t>(value / 2)
      : -static_cast<int32_t>(value / 2) - 1;
}

__device__ bool read_varint_order0(
    const uint8_t* body, int64_t length, int64_t* cursor, uint32_t* value) {
  uint32_t result = 0;
  int shift = 0;
  for (int used = 0; used < 5; ++used) {
    if (*cursor >= length) return false;
    const uint8_t byte = body[(*cursor)++];
    if (used == 4 && byte > 0x0f) return false;
    result |= static_cast<uint32_t>(byte & 0x7f) << shift;
    if ((byte & 0x80) == 0) {
      const int canonical = result < (1u << 7) ? 1 : result < (1u << 14) ? 2 :
          result < (1u << 21) ? 3 : result < (1u << 28) ? 4 : 5;
      if (canonical != used + 1) return false;
      *value = result;
      return true;
    }
    shift += 7;
  }
  return false;
}

__global__ void decode_order0_shards_kernel(
    const uint8_t* encoded,
    const int64_t* metadata,
    int64_t shard_count,
    uint32_t max_folded,
    uint32_t* folded,
    int32_t* status) {
  const int64_t shard = blockIdx.x;
  if (shard >= shard_count) return;
  if (metadata[shard * 5 + 0] != kEntropyOrder0) {
    decode_legacy_shard(encoded, metadata, shard, max_folded, folded, status);
    return;
  }
  const int64_t body_offset = metadata[shard * 5 + 1];
  const int64_t body_length = metadata[shard * 5 + 2];
  const int64_t sample_count = metadata[shard * 5 + 3];
  const int64_t output_offset = metadata[shard * 5 + 4];
  const uint8_t* body = encoded + body_offset;
  uint32_t* output = folded + output_offset;
  __shared__ uint16_t values[kRansAlphabet];
  __shared__ uint16_t frequency[kRansAlphabet];
  __shared__ uint16_t cumulative[kRansAlphabet];
  __shared__ uint8_t table[1 << kRansTableLog];
  __shared__ uint32_t states[4];
  __shared__ int lane_start[4];
  __shared__ int lane_length[4];
  __shared__ int symbol_count;
  __shared__ int valid;
  if (threadIdx.x == 0) {
    valid = 1;
    int64_t cursor = 0;
    if (body_length < 1 || body[cursor++] != kRansTableLog) valid = 0;
    uint32_t n = 0;
    if (valid && !read_varint_order0(body, body_length, &cursor, &n)) valid = 0;
    if (n == 0 || n > 255) valid = 0;
    symbol_count = static_cast<int>(n);
    uint32_t previous = 0;
    uint32_t assigned = 0;
    for (uint32_t i = 0; valid && i < n; ++i) {
      uint32_t delta = 0;
      if (!read_varint_order0(body, body_length, &cursor, &delta) || (i != 0 && delta == 0) ||
          previous + delta > max_folded) {
        valid = 0;
        break;
      }
      const uint32_t value = previous + delta;
      uint32_t freq = (1u << kRansTableLog) - assigned;
      if (i + 1 != n && !read_varint_order0(body, body_length, &cursor, &freq)) {
        valid = 0;
        break;
      }
      if (freq == 0 || assigned + freq > (1u << kRansTableLog) ||
          (i + 1 != n && assigned + freq == (1u << kRansTableLog))) {
        valid = 0;
        break;
      }
      values[i] = static_cast<uint16_t>(value);
      frequency[i] = static_cast<uint16_t>(freq);
      cumulative[i] = static_cast<uint16_t>(assigned);
      assigned += freq;
      previous = value;
    }
    if (assigned != (1u << kRansTableLog)) valid = 0;
    int total_payload = 0;
    for (int lane = 0; valid && lane < 3; ++lane) {
      if (cursor + 4 > body_length) {
        valid = 0;
      } else {
        lane_length[lane] = static_cast<int>(read_u32(body + cursor));
        total_payload += lane_length[lane];
        cursor += 4;
      }
    }
    if (valid && cursor + 16 > body_length) valid = 0;
    for (int lane = 0; valid && lane < 4; ++lane) {
      states[lane] = read_u32(body + cursor + lane * 4);
      if (states[lane] < kRansByteL) valid = 0;
    }
    cursor += valid ? 16 : 0;
    if (valid) {
      lane_length[3] = static_cast<int>(body_length - cursor - total_payload);
      if (lane_length[3] < 0) valid = 0;
      for (int lane = 0; lane < 4; ++lane) {
        lane_start[lane] = static_cast<int>(cursor);
        cursor += lane_length[lane];
      }
      if (cursor != body_length) valid = 0;
    }
    if (!valid) set_error(status, 5);
  }
  __syncthreads();
  if (!valid) return;
  for (int symbol = threadIdx.x; symbol < symbol_count; symbol += blockDim.x) {
    const int begin = cumulative[symbol];
    const int end = begin + frequency[symbol];
    for (int slot = begin; slot < end; ++slot) table[slot] = static_cast<uint8_t>(symbol);
  }
  __syncthreads();
  if (threadIdx.x < 4) {
    const int lane = threadIdx.x;
    uint32_t state = states[lane];
    int cursor = 0;
    for (int64_t index = lane; index < sample_count; index += 4) {
      const uint32_t slot = state & ((1u << kRansTableLog) - 1);
      const uint16_t symbol = table[slot];
      output[index] = values[symbol];
      state = static_cast<uint32_t>(frequency[symbol]) * (state >> kRansTableLog) +
          slot - cumulative[symbol];
      while (state < kRansByteL) {
        if (cursor >= lane_length[lane]) {
          set_error(status, 5);
          return;
        }
        state = (state << 8) | body[lane_start[lane] + cursor++];
      }
    }
    if (cursor != lane_length[lane] || state != kRansByteL) set_error(status, 5);
  }
}

__global__ void reconstruct_tiles_kernel(
    const uint32_t* folded,
    const int64_t* metadata,
    int64_t tile_count,
    int32_t step,
    int32_t max_sample,
    bool use_med,
    uint16_t* output) {
  const int64_t tile = blockIdx.x;
  if (tile >= tile_count) {
    return;
  }
  const int64_t plane_base = metadata[tile * 7 + 0];
  const int64_t plane_width = metadata[tile * 7 + 1];
  const int64_t origin_x = metadata[tile * 7 + 2];
  const int64_t origin_y = metadata[tile * 7 + 3];
  const int64_t width = metadata[tile * 7 + 4];
  const int64_t height = metadata[tile * 7 + 5];
  const int64_t folded_base = metadata[tile * 7 + 6];

  for (int64_t diagonal = 0; diagonal < width + height - 1; ++diagonal) {
    const int64_t diagonal_begin = diagonal - width + 1;
    const int64_t y_begin = diagonal_begin > 0 ? diagonal_begin : 0;
    const int64_t y_end = diagonal < height - 1 ? diagonal : height - 1;
    const int64_t count = y_end - y_begin + 1;
    for (int64_t item = threadIdx.x; item < count; item += blockDim.x) {
      const int64_t y = y_begin + item;
      const int64_t x = diagonal - y;
      const int64_t destination = plane_base + (origin_y + y) * plane_width + origin_x + x;
      const uint16_t left = x > 0 ? output[destination - 1] : 0;
      const uint16_t above = y > 0 ? output[destination - plane_width] : 0;
      const uint16_t upper_left = x > 0 && y > 0 ? output[destination - plane_width - 1] : 0;
      int32_t prediction;
      if (use_med) {
        if (upper_left >= max(left, above)) {
          prediction = min(left, above);
        } else if (upper_left <= min(left, above)) {
          prediction = max(left, above);
        } else {
          prediction = static_cast<int32_t>(left) + static_cast<int32_t>(above) -
              static_cast<int32_t>(upper_left);
        }
      } else {
        prediction = static_cast<int32_t>(left) + static_cast<int32_t>(above) -
            static_cast<int32_t>(upper_left);
        prediction = max(0, min(max_sample, prediction));
      }
      int32_t reconstructed = prediction +
          unzigzag(folded[folded_base + y * width + x]) * step;
      reconstructed = max(0, min(max_sample, reconstructed));
      output[destination] = static_cast<uint16_t>(reconstructed);
    }
    __syncthreads();
  }
}

__global__ void reconstruct_tiles_serial_kernel(
    const uint32_t* folded,
    const int64_t* metadata,
    int64_t tile_count,
    int32_t step,
    int32_t max_sample,
    bool use_med,
    uint16_t* output) {
  const int64_t tile = static_cast<int64_t>(blockIdx.x) * blockDim.x + threadIdx.x;
  if (tile >= tile_count) {
    return;
  }
  const int64_t plane_base = metadata[tile * 7 + 0];
  const int64_t plane_width = metadata[tile * 7 + 1];
  const int64_t origin_x = metadata[tile * 7 + 2];
  const int64_t origin_y = metadata[tile * 7 + 3];
  const int64_t width = metadata[tile * 7 + 4];
  const int64_t height = metadata[tile * 7 + 5];
  const int64_t folded_base = metadata[tile * 7 + 6];
  for (int64_t y = 0; y < height; ++y) {
    uint16_t left = 0;
    uint16_t upper_left = 0;
    for (int64_t x = 0; x < width; ++x) {
      const int64_t destination = plane_base + (origin_y + y) * plane_width + origin_x + x;
      const uint16_t above = y > 0 ? output[destination - plane_width] : 0;
      int32_t prediction;
      if (use_med) {
        if (upper_left >= max(left, above)) {
          prediction = min(left, above);
        } else if (upper_left <= min(left, above)) {
          prediction = max(left, above);
        } else {
          prediction = static_cast<int32_t>(left) + static_cast<int32_t>(above) -
              static_cast<int32_t>(upper_left);
        }
      } else {
        prediction = static_cast<int32_t>(left) + static_cast<int32_t>(above) -
            static_cast<int32_t>(upper_left);
        prediction = max(0, min(max_sample, prediction));
      }
      int32_t reconstructed = prediction +
          unzigzag(folded[folded_base + y * width + x]) * step;
      reconstructed = max(0, min(max_sample, reconstructed));
      output[destination] = static_cast<uint16_t>(reconstructed);
      upper_left = above;
      left = static_cast<uint16_t>(reconstructed);
    }
  }
}

}  // namespace

std::vector<torch::Tensor> fastvid_decode_cuda(
    const torch::Tensor& encoded,
    torch::Tensor shard_meta,
    const torch::Tensor& tile_meta,
    const torch::Tensor& tile_parse_meta,
    bool parse_metadata,
    int64_t total_samples,
    int64_t y_samples,
    int64_t secondary_samples,
    int64_t width,
    int64_t height,
    int64_t step,
    int64_t max_sample,
    bool grayscale,
    bool use_med,
    bool wavefront) {
  c10::cuda::CUDAGuard guard(encoded.device());
  auto folded = torch::empty({total_samples}, encoded.options().dtype(torch::kUInt32));
  auto output = torch::empty({total_samples}, encoded.options().dtype(torch::kUInt16));
  auto status = torch::zeros({1}, encoded.options().dtype(torch::kInt32));
  const auto stream = at::cuda::getCurrentCUDAStream(encoded.device().index());

  if (parse_metadata) {
    parse_metadata_kernel<<<tile_meta.size(0), 1, 0, stream>>>(
        encoded.data_ptr<uint8_t>(), encoded.numel(), tile_meta.data_ptr<int64_t>(),
        tile_parse_meta.data_ptr<int64_t>(), tile_meta.size(0),
        shard_meta.data_ptr<int64_t>(), status.data_ptr<int32_t>());
    C10_CUDA_KERNEL_LAUNCH_CHECK();
  }

  decode_order0_shards_kernel<<<shard_meta.size(0), 128, 0, stream>>>(
      encoded.data_ptr<uint8_t>(), shard_meta.data_ptr<int64_t>(), shard_meta.size(0),
      static_cast<uint32_t>(max_sample * 2), folded.data_ptr<uint32_t>(),
      status.data_ptr<int32_t>());
  C10_CUDA_KERNEL_LAUNCH_CHECK();
  if (wavefront) {
    reconstruct_tiles_kernel<<<tile_meta.size(0), 128, 0, stream>>>(
        folded.data_ptr<uint32_t>(),
        tile_meta.data_ptr<int64_t>(),
        tile_meta.size(0),
        static_cast<int32_t>(step),
        static_cast<int32_t>(max_sample),
        use_med,
        output.data_ptr<uint16_t>());
  } else {
    constexpr int threads = 256;
    const int blocks = static_cast<int>((tile_meta.size(0) + threads - 1) / threads);
    reconstruct_tiles_serial_kernel<<<blocks, threads, 0, stream>>>(
        folded.data_ptr<uint32_t>(),
        tile_meta.data_ptr<int64_t>(),
        tile_meta.size(0),
        static_cast<int32_t>(step),
        static_cast<int32_t>(max_sample),
        use_med,
        output.data_ptr<uint16_t>());
  }
  C10_CUDA_KERNEL_LAUNCH_CHECK();

  const int32_t host_status = status.cpu().item<int32_t>();
  TORCH_CHECK(host_status == 0, "malformed entropy payload (CUDA status ", host_status, ")");
  std::vector<torch::Tensor> planes;
  planes.push_back(output.narrow(0, 0, y_samples).view({height, width}));
  if (!grayscale) {
    const int64_t secondary_width = secondary_samples / height;
    planes.push_back(output.narrow(0, y_samples, secondary_samples)
                         .view({height, secondary_width}));
    planes.push_back(output.narrow(0, y_samples + secondary_samples, secondary_samples)
                         .view({height, secondary_width}));
  }
  return planes;
}
