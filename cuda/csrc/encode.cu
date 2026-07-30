#include <torch/extension.h>
#include <ATen/cuda/CUDAContext.h>
#include <c10/cuda/CUDAGuard.h>
#include <c10/cuda/CUDAException.h>

#include <algorithm>
#include <cstdint>
#include <cstring>
#include <limits>
#include <vector>

namespace {

constexpr int64_t kHeaderBytes = 32;
constexpr uint8_t kBitstreamVersion = 1;
constexpr int64_t kDirectoryEntryBytes = 32;
constexpr int64_t kShardSymbols = 4096;
constexpr int64_t kBlockSymbols = 128;
constexpr int kMaxRiceParameter = 16;
constexpr uint8_t kEntropyZeroRun = 0;
constexpr uint8_t kEntropyRiceBase = 1;
constexpr uint8_t kEntropyBlockPack = 18;
constexpr uint8_t kEntropyParallelShards = 19;
constexpr uint8_t kEntropyOrder0 = 19;
constexpr int kRansAlphabet = 511;
constexpr int kRansTableLog = 12;
constexpr uint32_t kRansByteL = 1u << 23;
constexpr uint8_t kPredictFullTileClampGradient = 6;

struct Tile {
  int64_t plane;
  int64_t x;
  int64_t y;
  int64_t width;
  int64_t height;
  int64_t folded_base;
  int64_t first_shard;
  int64_t shard_count;
};

std::vector<Tile> expected_tiles(
    int64_t width,
    int64_t height,
    int64_t tile_width,
    int64_t tile_height,
    int64_t layout) {
  std::vector<Tile> result;
  int64_t folded_base = 0;
  int64_t first_shard = 0;
  const int64_t plane_count = layout == 0 ? 1 : 3;
  for (int64_t plane = 0; plane < plane_count; ++plane) {
    const bool subsampled = layout == 1 && plane != 0;
    const int64_t plane_width = subsampled ? (width + 1) / 2 : width;
    const int64_t nominal_width = subsampled ? (tile_width + 1) / 2 : tile_width;
    for (int64_t y = 0; y < height; y += tile_height) {
      for (int64_t x = 0; x < plane_width; x += nominal_width) {
        const int64_t actual_width = std::min(nominal_width, plane_width - x);
        const int64_t actual_height = std::min(tile_height, height - y);
        const int64_t samples = actual_width * actual_height;
        const int64_t shards = (samples + kShardSymbols - 1) / kShardSymbols;
        result.push_back(Tile{
            plane, x, y, actual_width, actual_height, folded_base, first_shard, shards});
        folded_base += samples;
        first_shard += shards;
      }
    }
  }
  return result;
}

int32_t quantization_step(int64_t layout, int64_t bit_depth, int64_t quality) {
  if ((layout == 0 && bit_depth == 8) || (layout == 1 && bit_depth == 8)) {
    return 1;
  }
  int32_t denominator = 6;
  if (layout == 1 && bit_depth == 10) {
    denominator = 20;
  } else if (layout == 1 && bit_depth == 16) {
    denominator = 24;
  } else if (layout == 2 && bit_depth == 10) {
    denominator = 10;
  }
  const int32_t scale = int32_t{1} << (bit_depth - 8);
  return 1 + ((100 - quality) * scale + denominator - 1) / denominator;
}

int32_t refined_gray16_step(int64_t quality) {
  return 1 + ((100 - quality) * 256 + 7) / 8;
}

void put_u16(std::vector<uint8_t>& output, uint16_t value) {
  output.push_back(static_cast<uint8_t>(value));
  output.push_back(static_cast<uint8_t>(value >> 8));
}

void put_u32(std::vector<uint8_t>& output, uint32_t value) {
  for (int shift = 0; shift < 32; shift += 8) {
    output.push_back(static_cast<uint8_t>(value >> shift));
  }
}

void put_u64(std::vector<uint8_t>& output, uint64_t value) {
  for (int shift = 0; shift < 64; shift += 8) {
    output.push_back(static_cast<uint8_t>(value >> shift));
  }
}

__device__ int varint_length(uint32_t value) {
  return value < (1u << 7) ? 1
      : value < (1u << 14) ? 2
      : value < (1u << 21) ? 3
      : value < (1u << 28) ? 4
      : 5;
}

__device__ int write_varint(uint8_t* output, uint32_t value);

__device__ uint32_t zigzag(int32_t value) {
  return value >= 0
      ? static_cast<uint32_t>(value) * 2
      : static_cast<uint32_t>(-value) * 2 - 1;
}

__device__ void set_error(int32_t* status, int32_t code) {
  atomicCAS(status, 0, code);
}

__global__ void predict_tiles_kernel(
    const uint16_t* source,
    const int64_t* metadata,
    int64_t tile_count,
    int32_t step,
    int32_t max_sample,
    bool use_med,
    uint16_t* reconstructed,
    uint32_t* folded,
    int32_t* status) {
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
      const int64_t index = plane_base + (origin_y + y) * plane_width + origin_x + x;
      const uint16_t sample = source[index];
      if (sample > max_sample) {
        set_error(status, 1);
      }
      const uint16_t left = x > 0 ? reconstructed[index - 1] : 0;
      const uint16_t above = y > 0 ? reconstructed[index - plane_width] : 0;
      const uint16_t upper_left = x > 0 && y > 0
          ? reconstructed[index - plane_width - 1]
          : 0;
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
      const int32_t residual = static_cast<int32_t>(sample) - prediction;
      const int32_t magnitude = (abs(residual) + step / 2) / step;
      const int32_t quantized = residual < 0 ? -magnitude : magnitude;
      const int32_t reconstructed_value =
          max(0, min(max_sample, prediction + quantized * step));
      reconstructed[index] = static_cast<uint16_t>(reconstructed_value);
      folded[folded_base + y * width + x] = zigzag(quantized);
    }
    __syncthreads();
  }
}

__global__ void analyze_shards_kernel(
    const uint32_t* folded,
    const int64_t* shard_metadata,
    int64_t shard_count,
    int64_t* analysis,
    int32_t* status,
    bool skip_order0) {
  const int64_t shard = blockIdx.x;
  if (shard >= shard_count) {
    return;
  }
  if (skip_order0 && analysis[shard * 8 + 0] == kEntropyOrder0) return;
  const int64_t base = shard_metadata[shard * 3 + 0];
  const int count = static_cast<int>(shard_metadata[shard * 3 + 1]);
  const int lane_count = min(4, count);
  __shared__ unsigned long long lane_bits[(kMaxRiceParameter + 1) * 4];
  __shared__ uint32_t zero_cost[128];
  __shared__ uint32_t block_cost[kShardSymbols / kBlockSymbols];
  for (int index = threadIdx.x; index < (kMaxRiceParameter + 1) * 4;
       index += blockDim.x) {
    lane_bits[index] = 0;
  }
  zero_cost[threadIdx.x] = 0;
  if (threadIdx.x < kShardSymbols / kBlockSymbols) {
    block_cost[threadIdx.x] = 0;
  }
  __syncthreads();

  uint64_t local_lane_bits[kMaxRiceParameter + 1] = {};
  uint32_t local_zero_cost = 0;
  const int lane = threadIdx.x % lane_count;
  for (int index = threadIdx.x; index < count; index += blockDim.x) {
    const uint32_t value = folded[base + index];
    for (int parameter = 0; parameter <= kMaxRiceParameter; ++parameter) {
      local_lane_bits[parameter] +=
          static_cast<uint64_t>(value >> parameter) + 1 + parameter;
    }
    if (value != 0) {
      local_zero_cost += varint_length(value * 2 - 1);
    } else if (index == 0 || folded[base + index - 1] != 0) {
      uint32_t run = 1;
      while (index + run < count && folded[base + index + run] == 0) {
        ++run;
      }
      local_zero_cost += varint_length((run - 1) * 2);
    }
  }
  for (int parameter = 0; parameter <= kMaxRiceParameter; ++parameter) {
    atomicAdd(
        &lane_bits[parameter * 4 + lane],
        static_cast<unsigned long long>(local_lane_bits[parameter]));
  }
  zero_cost[threadIdx.x] = local_zero_cost;

  if (threadIdx.x < (count + kBlockSymbols - 1) / kBlockSymbols) {
    const int start = threadIdx.x * kBlockSymbols;
    const int remaining = count - start;
    const int symbols = remaining < kBlockSymbols ? remaining : kBlockSymbols;
    uint32_t maximum = 0;
    for (int index = 0; index < symbols; ++index) {
      maximum = max(maximum, folded[base + start + index]);
    }
    const int width = maximum == 0 ? 0 : 32 - __clz(maximum);
    block_cost[threadIdx.x] = 1 + (symbols * width + 7) / 8;
  }
  __syncthreads();
  for (int stride = blockDim.x / 2; stride > 0; stride >>= 1) {
    if (threadIdx.x < stride) {
      zero_cost[threadIdx.x] += zero_cost[threadIdx.x + stride];
    }
    __syncthreads();
  }
  if (threadIdx.x != 0) {
    return;
  }
  const int64_t zero_bytes = zero_cost[0];

  int best_parameter = 0;
  int64_t best_lane_bytes = std::numeric_limits<int64_t>::max();
  int64_t rice_body_bytes = 0;
  for (int parameter = 0; parameter <= kMaxRiceParameter; ++parameter) {
    int64_t bytes = 0;
    for (int lane = 0; lane < lane_count; ++lane) {
      bytes += static_cast<int64_t>((lane_bits[parameter * 4 + lane] + 7) / 8);
    }
    if (bytes < best_lane_bytes) {
      best_lane_bytes = bytes;
      best_parameter = parameter;
    }
  }
  rice_body_bytes = (lane_count - 1) * 4 + best_lane_bytes;

  int64_t block_bytes = 0;
  const int block_count = (count + kBlockSymbols - 1) / kBlockSymbols;
  for (int block = 0; block < block_count; ++block) {
    block_bytes += block_cost[block];
  }

  int mode;
  int64_t body_bytes;
  if (block_bytes < zero_bytes && block_bytes < rice_body_bytes) {
    mode = kEntropyBlockPack;
    body_bytes = block_bytes;
  } else if (zero_bytes <= rice_body_bytes) {
    mode = kEntropyZeroRun;
    body_bytes = zero_bytes;
  } else {
    mode = kEntropyRiceBase + best_parameter;
    body_bytes = rice_body_bytes;
  }
  if (body_bytes > std::numeric_limits<uint16_t>::max()) {
    set_error(status, 2);
  }
  analysis[shard * 8 + 0] = mode;
  analysis[shard * 8 + 1] = best_parameter;
  analysis[shard * 8 + 2] = body_bytes;
  analysis[shard * 8 + 3] = body_bytes + 3;
  for (int lane = 0; lane < 4; ++lane) {
    analysis[shard * 8 + 4 + lane] = lane < lane_count
        ? static_cast<int64_t>((lane_bits[best_parameter * 4 + lane] + 7) / 8)
        : 0;
  }
}

__global__ void analyze_order0_shards_kernel(
    const uint32_t* folded,
    const int64_t* shard_metadata,
    int64_t shard_count,
    int64_t* analysis,
    uint8_t* rans_scratch,
    uint32_t* rans_states,
    bool force_order0) {
  const int64_t shard = blockIdx.x;
  if (shard >= shard_count) return;
  const int64_t base = shard_metadata[shard * 3 + 0];
  const int count = static_cast<int>(shard_metadata[shard * 3 + 1]);
  __shared__ uint32_t histogram[kRansAlphabet];
  __shared__ uint16_t frequency[kRansAlphabet];
  __shared__ uint16_t cumulative[kRansAlphabet];
  __shared__ int supported;
  __shared__ int table_bytes;
  __shared__ int lane_bytes[4];
  for (int value = threadIdx.x; value < kRansAlphabet; value += blockDim.x) {
    histogram[value] = 0;
    frequency[value] = 0;
    cumulative[value] = 0;
  }
  if (threadIdx.x == 0) supported = 1;
  __syncthreads();
  for (int index = threadIdx.x; index < count; index += blockDim.x) {
    const uint32_t value = folded[base + index];
    if (value >= kRansAlphabet) {
      atomicExch(&supported, 0);
    } else {
      atomicAdd(&histogram[value], 1u);
    }
  }
  __syncthreads();
  if (!supported) {
    if (threadIdx.x == 0 && force_order0) analysis[shard * 8 + 0] = -1;
    return;
  }
  if (threadIdx.x == 0) {
    int distinct = 0;
    for (int value = 0; value < kRansAlphabet; ++value) distinct += histogram[value] != 0;
    if (distinct > 255) supported = 0;
    int cursor = 0;
    uint8_t* table_output = rans_scratch + shard * 10240;
    table_output[cursor++] = kRansTableLog;
    cursor += write_varint(table_output + cursor, distinct);
    int bytes = 1 + varint_length(distinct) + 12 + 16;
    uint32_t assigned = 0;
    uint32_t prefix = 0;
    int remaining_symbols = distinct;
    int previous = 0;
    for (int value = 0; value < kRansAlphabet; ++value) {
      if (histogram[value] == 0) continue;
      --remaining_symbols;
      prefix += histogram[value];
      uint32_t end;
      if (remaining_symbols == 0) {
        end = 1u << kRansTableLog;
      } else {
        end = static_cast<uint32_t>((static_cast<uint64_t>(prefix) << kRansTableLog) / count);
        end = max(end, assigned + 1);
        end = min(end, (1u << kRansTableLog) - static_cast<uint32_t>(remaining_symbols));
      }
      frequency[value] = static_cast<uint16_t>(end - assigned);
      cumulative[value] = static_cast<uint16_t>(assigned);
      bytes += varint_length(static_cast<uint32_t>(value - previous));
      cursor += write_varint(table_output + cursor, static_cast<uint32_t>(value - previous));
      if (remaining_symbols != 0) bytes += varint_length(end - assigned);
      if (remaining_symbols != 0) cursor += write_varint(table_output + cursor, end - assigned);
      assigned = end;
      previous = value;
    }
    table_bytes = bytes;
  }
  __syncthreads();
  if (!supported) {
    if (threadIdx.x == 0 && force_order0) analysis[shard * 8 + 0] = -1;
    return;
  }
  if (threadIdx.x < 4) {
    const int lane = threadIdx.x;
    uint32_t state = kRansByteL;
    int bytes = 0;
    int index = count - 1 - ((count - 1 - lane) & 3);
    for (; index >= 0; index -= 4) {
      const uint32_t value = folded[base + index];
      const uint32_t freq = frequency[value];
      const uint64_t threshold =
          (static_cast<uint64_t>(kRansByteL >> kRansTableLog) << 8) * freq;
      while (state >= threshold) {
        rans_scratch[shard * 10240 + 2048 + lane * 2048 + bytes] =
            static_cast<uint8_t>(state);
        ++bytes;
        state >>= 8;
      }
      state = ((state / freq) << kRansTableLog) + state % freq + cumulative[value];
    }
    lane_bytes[lane] = bytes;
    rans_states[shard * 4 + lane] = state;
  }
  __syncthreads();
  if (threadIdx.x == 0) {
    const int64_t body_bytes = table_bytes + lane_bytes[0] + lane_bytes[1] +
        lane_bytes[2] + lane_bytes[3];
    if (force_order0 || body_bytes < analysis[shard * 8 + 2]) {
      analysis[shard * 8 + 0] = kEntropyOrder0;
      analysis[shard * 8 + 1] = kRansTableLog;
      analysis[shard * 8 + 2] = body_bytes;
      analysis[shard * 8 + 3] = body_bytes + 3;
      for (int lane = 0; lane < 4; ++lane) analysis[shard * 8 + 4 + lane] = lane_bytes[lane];
    }
  }
}

__device__ void write_u16(uint8_t* output, uint16_t value) {
  output[0] = static_cast<uint8_t>(value);
  output[1] = static_cast<uint8_t>(value >> 8);
}

__device__ void write_u32(uint8_t* output, uint32_t value) {
  output[0] = static_cast<uint8_t>(value);
  output[1] = static_cast<uint8_t>(value >> 8);
  output[2] = static_cast<uint8_t>(value >> 16);
  output[3] = static_cast<uint8_t>(value >> 24);
}

__device__ int write_varint(uint8_t* output, uint32_t value) {
  int bytes = 0;
  do {
    uint8_t byte = static_cast<uint8_t>(value & 0x7f);
    value >>= 7;
    if (value != 0) {
      byte |= 0x80;
    }
    output[bytes++] = byte;
  } while (value != 0);
  return bytes;
}

__device__ void set_stream_bit(uint8_t* output, int64_t bit_position) {
  output[bit_position >> 3] |= static_cast<uint8_t>(1u << (bit_position & 7));
}

__device__ void set_stream_bit_atomic(uint8_t* output, int64_t bit_position) {
  const uintptr_t address = reinterpret_cast<uintptr_t>(output + (bit_position >> 3));
  const uintptr_t aligned = address & ~uintptr_t{3};
  const int bit = static_cast<int>((address - aligned) * 8 + (bit_position & 7));
  atomicOr(reinterpret_cast<unsigned int*>(aligned), 1u << bit);
}

__device__ void set_stream_byte_atomic(uint8_t* output, uint8_t value) {
  const uintptr_t address = reinterpret_cast<uintptr_t>(output);
  const uintptr_t aligned = address & ~uintptr_t{3};
  const int shift = static_cast<int>(address - aligned) * 8;
  atomicOr(reinterpret_cast<unsigned int*>(aligned), static_cast<unsigned int>(value) << shift);
}

__device__ void write_stream_bits(
    uint8_t* output,
    int64_t bit_position,
    uint32_t value,
    int count) {
  for (int bit = 0; bit < count; ++bit) {
    if ((value >> bit) & 1u) {
      set_stream_bit(output, bit_position + bit);
    }
  }
}

__global__ void emit_shards_kernel(
    const uint32_t* folded,
    const int64_t* shard_metadata,
    const int64_t* analysis,
    const int64_t* output_offsets,
    int64_t shard_count,
    const uint8_t* rans_scratch,
    const uint32_t* rans_states,
    uint8_t* output) {
  const int64_t shard = blockIdx.x;
  if (shard >= shard_count) {
    return;
  }
  const int64_t folded_base = shard_metadata[shard * 3 + 0];
  const int count = static_cast<int>(shard_metadata[shard * 3 + 1]);
  const int mode = static_cast<int>(analysis[shard * 8 + 0]);
  const int parameter = static_cast<int>(analysis[shard * 8 + 1]);
  const int body_bytes = static_cast<int>(analysis[shard * 8 + 2]);
  uint8_t* shard_output = output + output_offsets[shard];
  uint8_t* body = shard_output + 3;
  if (threadIdx.x == 0) {
    shard_output[0] = static_cast<uint8_t>(mode);
    write_u16(shard_output + 1, static_cast<uint16_t>(body_bytes));
  }
  __syncthreads();

  if (mode == kEntropyZeroRun) {
    if (threadIdx.x != 0) {
      return;
    }
    int cursor = 0;
    uint32_t run = 0;
    for (int index = 0; index < count; ++index) {
      const uint32_t value = folded[folded_base + index];
      if (value == 0) {
        ++run;
      } else {
        if (run != 0) {
          cursor += write_varint(body + cursor, (run - 1) * 2);
          run = 0;
        }
        cursor += write_varint(body + cursor, value * 2 - 1);
      }
    }
    if (run != 0) {
      write_varint(body + cursor, (run - 1) * 2);
    }
    return;
  }

  if (mode >= kEntropyRiceBase && mode <= kEntropyRiceBase + kMaxRiceParameter) {
    const int lane_count = min(4, count);
    const int lane = threadIdx.x / 32;
    const int warp_thread = threadIdx.x & 31;
    int64_t lane_offset = (lane_count - 1) * 4;
    for (int prior = 0; prior < lane; ++prior) {
      lane_offset += analysis[shard * 8 + 4 + prior];
    }
    if (lane < lane_count && warp_thread == 0 && lane + 1 < lane_count) {
      write_u32(
          body + lane * 4,
          static_cast<uint32_t>(analysis[shard * 8 + 4 + lane]));
    }
    __syncthreads();
    if (lane >= lane_count) {
      return;
    }
    uint8_t* lane_output = body + lane_offset;
    const uint32_t remainder_mask = parameter == 0 ? 0 : (1u << parameter) - 1;
    const int lane_symbols = (count - lane + lane_count - 1) / lane_count;
    int64_t group_base = 0;
    for (int group = 0; group * 32 < lane_symbols; ++group) {
      const int lane_index = group * 32 + warp_thread;
      const bool valid = lane_index < lane_symbols;
      const int index = lane + lane_index * lane_count;
      const uint32_t value = valid ? folded[folded_base + index] : 0;
      const uint32_t quotient = value >> parameter;
      const int64_t code_bits = valid ? static_cast<int64_t>(quotient) + 1 + parameter : 0;
      int64_t inclusive = code_bits;
      for (int offset = 1; offset < 32; offset <<= 1) {
        const int64_t prior = __shfl_up_sync(0xffffffff, inclusive, offset);
        if (warp_thread >= offset) {
          inclusive += prior;
        }
      }
      if (valid) {
        const int64_t code_start = group_base + inclusive - code_bits;
        set_stream_bit_atomic(lane_output, code_start + quotient);
        for (int bit = 0; bit < parameter; ++bit) {
          if ((value >> bit) & 1u) {
            set_stream_bit_atomic(lane_output, code_start + quotient + 1 + bit);
          }
        }
      }
      group_base += __shfl_sync(0xffffffff, inclusive, 31);
    }
    return;
  }

  if (mode == kEntropyOrder0) {
    const int lane0 = static_cast<int>(analysis[shard * 8 + 4]);
    const int lane1 = static_cast<int>(analysis[shard * 8 + 5]);
    const int lane2 = static_cast<int>(analysis[shard * 8 + 6]);
    const int lane3 = static_cast<int>(analysis[shard * 8 + 7]);
    const int sparse_bytes = body_bytes - lane0 - lane1 - lane2 - lane3 - 28;
    for (int index = threadIdx.x; index < sparse_bytes; index += blockDim.x) {
      body[index] = rans_scratch[shard * 10240 + index];
    }
    if (threadIdx.x < 3) {
      write_u32(body + sparse_bytes + threadIdx.x * 4,
          static_cast<uint32_t>(analysis[shard * 8 + 4 + threadIdx.x]));
    }
    if (threadIdx.x < 4) {
      const int lane = threadIdx.x;
      write_u32(body + sparse_bytes + 12 + lane * 4, rans_states[shard * 4 + lane]);
      int start = sparse_bytes + 28;
      for (int prior = 0; prior < lane; ++prior) {
        start += static_cast<int>(analysis[shard * 8 + 4 + prior]);
      }
      const int length = static_cast<int>(analysis[shard * 8 + 4 + lane]);
      const uint8_t* source = rans_scratch + shard * 10240 + 2048 + lane * 2048;
      for (int index = 0; index < length; ++index) body[start + index] = source[length - 1 - index];
    }
  }
}

__global__ void emit_block_shards_kernel(
    const uint32_t* folded,
    const int64_t* shard_metadata,
    const int64_t* analysis,
    const int64_t* output_offsets,
    const int64_t* block_shards,
    int64_t block_shard_count,
    uint8_t* output) {
  const int64_t list_index = blockIdx.x;
  if (list_index >= block_shard_count) {
    return;
  }
  const int64_t shard = block_shards[list_index];
  const int64_t folded_base = shard_metadata[shard * 3 + 0];
  const int count = static_cast<int>(shard_metadata[shard * 3 + 1]);
  const int body_bytes = static_cast<int>(analysis[shard * 8 + 2]);
  uint8_t* body = output + output_offsets[shard] + 3;
  const int block = threadIdx.x / 32;
  const int lane = threadIdx.x & 31;
  const int block_count = (count + kBlockSymbols - 1) / kBlockSymbols;
  __shared__ uint32_t block_cost[kShardSymbols / kBlockSymbols];
  __shared__ uint8_t block_width[kShardSymbols / kBlockSymbols];
  if (block < block_count) {
    const int start = block * kBlockSymbols;
    const int symbols = min(static_cast<int>(kBlockSymbols), count - start);
    uint32_t maximum = 0;
    for (int index = lane; index < symbols; index += 32) {
      maximum = max(maximum, folded[folded_base + start + index]);
    }
    for (int offset = 16; offset > 0; offset >>= 1) {
      maximum = max(maximum, __shfl_down_sync(0xffffffff, maximum, offset));
    }
    if (lane == 0) {
      const int width = maximum == 0 ? 0 : 32 - __clz(maximum);
      block_width[block] = static_cast<uint8_t>(width);
      block_cost[block] = 1 + (symbols * width + 7) / 8;
    }
  }
  __syncthreads();
  if (block >= block_count) {
    return;
  }
  int block_offset = 0;
  for (int prior = 0; prior < block; ++prior) {
    block_offset += block_cost[prior];
  }
  const int width = block_width[block];
  if (lane == 0) {
    set_stream_byte_atomic(body + block_offset, static_cast<uint8_t>(width));
  }
  uint8_t* packed = body + block_offset + 1;
  const int start = block * kBlockSymbols;
  const int symbols = min(static_cast<int>(kBlockSymbols), count - start);
  for (int index = lane; index < symbols; index += 32) {
    const uint32_t value = folded[folded_base + start + index];
    const int64_t bit_position = static_cast<int64_t>(index) * width;
    for (int bit = 0; bit < width; ++bit) {
      if ((value >> bit) & 1u) {
        set_stream_bit_atomic(packed, bit_position + bit);
      }
    }
  }
  if (block == block_count - 1 && lane == 0) {
    const int total = block_offset + block_cost[block];
    if (total != body_bytes) {
      asm("trap;");
    }
  }
}

}  // namespace

torch::Tensor fastvid_encode_cuda(
    std::vector<torch::Tensor> planes,
    int64_t layout,
    int64_t bit_depth,
    int64_t quality,
    int64_t fps_numerator,
    int64_t fps_denominator,
    int64_t tile_width,
    int64_t tile_height) {
  TORCH_CHECK(layout >= 0 && layout <= 2, "layout must be gray, yuv422, or rgb444");
  TORCH_CHECK(planes.size() == (layout == 0 ? 1 : 3),
              "plane count does not match layout");
  TORCH_CHECK(bit_depth == 8 || bit_depth == 10 || bit_depth == 12 || bit_depth == 16,
              "bit_depth must be 8, 10, 12, or 16");
  TORCH_CHECK(quality >= 1 && quality <= 100, "quality must be in 1..=100");
  TORCH_CHECK(fps_numerator > 0 && fps_numerator <= std::numeric_limits<uint32_t>::max() &&
                  fps_denominator > 0 && fps_denominator <= std::numeric_limits<uint32_t>::max(),
              "frame-rate components must be nonzero u32 values");
  TORCH_CHECK(tile_width > 0 && tile_width <= std::numeric_limits<uint16_t>::max() &&
                  tile_height > 0 && tile_height <= std::numeric_limits<uint16_t>::max(),
              "tile dimensions must be nonzero u16 values");
  TORCH_CHECK(planes[0].is_cuda(), "encoder input planes must be CUDA tensors");
  TORCH_CHECK(planes[0].scalar_type() == torch::kUInt16 && planes[0].dim() == 2,
              "encoder planes must be two-dimensional uint16 tensors");
  const auto device = planes[0].device();
  const int64_t height = planes[0].size(0);
  const int64_t width = planes[0].size(1);
  TORCH_CHECK(width > 0 && height > 0 &&
                  width <= std::numeric_limits<uint32_t>::max() &&
                  height <= std::numeric_limits<uint32_t>::max(),
              "frame dimensions must be nonzero u32 values");
  const int64_t secondary_width = layout == 1 ? (width + 1) / 2 : width;
  for (size_t plane = 0; plane < planes.size(); ++plane) {
    TORCH_CHECK(planes[plane].device() == device, "all planes must use the same CUDA device");
    TORCH_CHECK(planes[plane].scalar_type() == torch::kUInt16 && planes[plane].dim() == 2,
                "encoder planes must be two-dimensional uint16 tensors");
    const int64_t expected_width = plane == 0 ? width : secondary_width;
    TORCH_CHECK(planes[plane].size(0) == height && planes[plane].size(1) == expected_width,
                "plane dimensions do not match declared layout");
  }

  c10::cuda::CUDAGuard guard(device);
  std::vector<torch::Tensor> flat_planes;
  flat_planes.reserve(planes.size());
  for (const auto& plane : planes) {
    flat_planes.push_back(plane.contiguous().reshape({-1}));
  }
  auto source = torch::cat(flat_planes, 0);
  auto tiles = expected_tiles(width, height, tile_width, tile_height, layout);
  TORCH_CHECK(tiles.size() <= (1u << 20), "too many tiles");
  const int64_t total_samples = source.numel();
  const int64_t total_shards = tiles.empty() ? 0 : tiles.back().first_shard + tiles.back().shard_count;

  std::vector<int64_t> tile_metadata;
  std::vector<int64_t> shard_metadata;
  tile_metadata.reserve(tiles.size() * 7);
  shard_metadata.reserve(total_shards * 3);
  const int64_t y_samples = width * height;
  const int64_t secondary_samples = secondary_width * height;
  const int64_t plane_bases[3] = {0, y_samples, y_samples + secondary_samples};
  const int64_t plane_widths[3] = {width, secondary_width, secondary_width};
  for (size_t tile_index = 0; tile_index < tiles.size(); ++tile_index) {
    const auto& tile = tiles[tile_index];
    tile_metadata.insert(tile_metadata.end(), {
        plane_bases[tile.plane], plane_widths[tile.plane], tile.x, tile.y,
        tile.width, tile.height, tile.folded_base});
    const int64_t tile_samples = tile.width * tile.height;
    for (int64_t shard = 0; shard < tile.shard_count; ++shard) {
      const int64_t local = shard * kShardSymbols;
      shard_metadata.insert(shard_metadata.end(), {
          tile.folded_base + local,
          std::min(kShardSymbols, tile_samples - local),
          static_cast<int64_t>(tile_index)});
    }
  }
  auto long_cpu = torch::TensorOptions().dtype(torch::kInt64).device(torch::kCPU);
  auto tile_meta_cpu = torch::from_blob(
      tile_metadata.data(), {static_cast<int64_t>(tiles.size()), 7}, long_cpu).clone();
  auto shard_meta_cpu = torch::from_blob(
      shard_metadata.data(), {total_shards, 3}, long_cpu).clone();
  auto tile_meta = tile_meta_cpu.to(device);
  auto shard_meta = shard_meta_cpu.to(device);
  auto folded = torch::empty({total_samples}, source.options().dtype(torch::kUInt32));
  auto reconstructed = torch::zeros({total_samples}, source.options().dtype(torch::kUInt16));
  auto analysis = torch::empty({total_shards, 8}, source.options().dtype(torch::kInt64));
  auto rans_scratch = torch::empty({total_shards, 10240}, source.options().dtype(torch::kUInt8));
  auto rans_states = torch::empty({total_shards, 4}, source.options().dtype(torch::kUInt32));
  auto status = torch::zeros({1}, source.options().dtype(torch::kInt32));
  int32_t step = quantization_step(layout, bit_depth, quality);
  const int32_t max_sample = (int32_t{1} << bit_depth) - 1;
  const auto stream = at::cuda::getCurrentCUDAStream(device.index());

  const bool force_order0 = layout == 2 && (bit_depth != 10 || width < 3840);
  auto run_analysis = [&](int32_t active_step, bool use_med, bool reset) {
    if (reset) {
      reconstructed.zero_();
      status.zero_();
    }
    predict_tiles_kernel<<<tiles.size(), 256, 0, stream>>>(
        source.data_ptr<uint16_t>(), tile_meta.data_ptr<int64_t>(), tiles.size(),
        active_step, max_sample, use_med, reconstructed.data_ptr<uint16_t>(),
        folded.data_ptr<uint32_t>(), status.data_ptr<int32_t>());
    C10_CUDA_KERNEL_LAUNCH_CHECK();
    if (!force_order0) {
      analyze_shards_kernel<<<total_shards, 128, 0, stream>>>(
          folded.data_ptr<uint32_t>(), shard_meta.data_ptr<int64_t>(), total_shards,
          analysis.data_ptr<int64_t>(), status.data_ptr<int32_t>(), force_order0);
      C10_CUDA_KERNEL_LAUNCH_CHECK();
    }
    analyze_order0_shards_kernel<<<total_shards, 128, 0, stream>>>(
        folded.data_ptr<uint32_t>(), shard_meta.data_ptr<int64_t>(), total_shards,
        analysis.data_ptr<int64_t>(), rans_scratch.data_ptr<uint8_t>(),
        rans_states.data_ptr<uint32_t>(), force_order0);
    C10_CUDA_KERNEL_LAUNCH_CHECK();
    if (force_order0) {
      analyze_shards_kernel<<<total_shards, 128, 0, stream>>>(
          folded.data_ptr<uint32_t>(), shard_meta.data_ptr<int64_t>(), total_shards,
          analysis.data_ptr<int64_t>(), status.data_ptr<int32_t>(), force_order0);
      C10_CUDA_KERNEL_LAUNCH_CHECK();
    }
    auto result = analysis.cpu();
    const int32_t host_status = status.cpu().item<int32_t>();
    TORCH_CHECK(host_status == 0, "CUDA encoder analysis failed (status ", host_status, ")");
    return result;
  };

  auto analysis_cpu = run_analysis(step, layout != 0, false);
  const auto* analysis_data = analysis_cpu.data_ptr<int64_t>();
  int64_t baseline_payload_bytes = 0;
  for (int64_t shard = 0; shard < total_shards; ++shard) {
    baseline_payload_bytes += analysis_data[shard * 8 + 3];
  }
  const bool gray_med = layout == 0 && bit_depth == 10 && step > 1 &&
      baseline_payload_bytes * 10 > total_samples * 3;
  if (gray_med) {
    analysis_cpu = run_analysis(step, true, true);
    analysis_data = analysis_cpu.data_ptr<int64_t>();
  }
  bool refined_gray16 = false;
  if (layout == 0 && bit_depth == 16 && step > 1) {
    const bool direct_refinement = baseline_payload_bytes < total_samples / 10 ||
        baseline_payload_bytes > total_samples / 2;
    const bool probe_refinement = !direct_refinement &&
        baseline_payload_bytes < total_samples / 5;
    if (direct_refinement || probe_refinement) {
      const int32_t baseline_step = step;
      step = refined_gray16_step(quality);
      analysis_cpu = run_analysis(step, false, true);
      analysis_data = analysis_cpu.data_ptr<int64_t>();
      int64_t refined_payload_bytes = 0;
      for (int64_t shard = 0; shard < total_shards; ++shard) {
        refined_payload_bytes += analysis_data[shard * 8 + 3];
      }
      refined_gray16 = direct_refinement ||
          refined_payload_bytes < total_samples / 10;
      if (!refined_gray16) {
        step = baseline_step;
        analysis_cpu = run_analysis(step, false, true);
        analysis_data = analysis_cpu.data_ptr<int64_t>();
      }
    }
  }
  std::vector<int64_t> tile_lengths(tiles.size(), 0);
  std::vector<int64_t> block_shards;
  for (int64_t shard = 0; shard < total_shards; ++shard) {
    const int64_t tile_index = shard_metadata[shard * 3 + 2];
    tile_lengths[tile_index] += analysis_data[shard * 8 + 3];
    if (analysis_data[shard * 8 + 0] == kEntropyBlockPack) {
      block_shards.push_back(shard);
    }
  }

  const int64_t payload_start = kHeaderBytes + tiles.size() * kDirectoryEntryBytes;
  std::vector<int64_t> shard_offsets(total_shards);
  int64_t stream_length = payload_start;
  for (size_t tile_index = 0; tile_index < tiles.size(); ++tile_index) {
    for (int64_t shard = 0; shard < tiles[tile_index].shard_count; ++shard) {
      const int64_t index = tiles[tile_index].first_shard + shard;
      shard_offsets[index] = stream_length;
      stream_length += analysis_data[index * 8 + 3];
    }
  }
  TORCH_CHECK(stream_length > 0 && stream_length <= std::numeric_limits<int64_t>::max(),
              "encoded stream is too large");

  std::vector<uint8_t> prefix;
  prefix.reserve(payload_start);
  prefix.insert(prefix.end(), {'F', 'V', 'I', 'D'});
  prefix.push_back(kBitstreamVersion);
  prefix.push_back(static_cast<uint8_t>(layout));
  prefix.push_back(static_cast<uint8_t>(quality));
  prefix.push_back(static_cast<uint8_t>(
      (bit_depth - 8) | (gray_med ? 0x40 : 0) | (refined_gray16 ? 0x80 : 0)));
  put_u32(prefix, static_cast<uint32_t>(width));
  put_u32(prefix, static_cast<uint32_t>(height));
  put_u16(prefix, static_cast<uint16_t>(tile_width));
  put_u16(prefix, static_cast<uint16_t>(tile_height));
  put_u32(prefix, static_cast<uint32_t>(fps_numerator));
  put_u32(prefix, static_cast<uint32_t>(fps_denominator));
  put_u32(prefix, static_cast<uint32_t>(tiles.size()));
  int64_t tile_offset = payload_start;
  for (size_t tile_index = 0; tile_index < tiles.size(); ++tile_index) {
    const auto& tile = tiles[tile_index];
    prefix.push_back(static_cast<uint8_t>(tile.plane));
    prefix.push_back(kEntropyParallelShards);
    prefix.push_back(kPredictFullTileClampGradient);
    prefix.push_back(0);
    put_u32(prefix, static_cast<uint32_t>(tile.x));
    put_u32(prefix, static_cast<uint32_t>(tile.y));
    put_u32(prefix, static_cast<uint32_t>(tile.width));
    put_u32(prefix, static_cast<uint32_t>(tile.height));
    put_u64(prefix, static_cast<uint64_t>(tile_offset));
    TORCH_CHECK(tile_lengths[tile_index] <= std::numeric_limits<uint32_t>::max(),
                "tile payload is too large");
    put_u32(prefix, static_cast<uint32_t>(tile_lengths[tile_index]));
    tile_offset += tile_lengths[tile_index];
  }
  TORCH_CHECK(static_cast<int64_t>(prefix.size()) == payload_start,
              "internal CUDA encoder prefix size mismatch");

  auto output_storage = torch::zeros({stream_length + 3}, source.options().dtype(torch::kUInt8));
  auto output = output_storage.narrow(0, 0, stream_length);
  auto prefix_cpu = torch::from_blob(
      prefix.data(), {static_cast<int64_t>(prefix.size())},
      torch::TensorOptions().dtype(torch::kUInt8).device(torch::kCPU)).clone();
  output.narrow(0, 0, prefix.size()).copy_(prefix_cpu.to(device));
  auto offsets_cpu = torch::from_blob(
      shard_offsets.data(), {total_shards}, long_cpu).clone();
  auto offsets = offsets_cpu.to(device);
  emit_shards_kernel<<<total_shards, 128, 0, stream>>>(
      folded.data_ptr<uint32_t>(), shard_meta.data_ptr<int64_t>(), analysis.data_ptr<int64_t>(),
      offsets.data_ptr<int64_t>(), total_shards, rans_scratch.data_ptr<uint8_t>(),
      rans_states.data_ptr<uint32_t>(), output_storage.data_ptr<uint8_t>());
  C10_CUDA_KERNEL_LAUNCH_CHECK();
  if (!block_shards.empty()) {
    auto block_shards_cpu = torch::from_blob(
        block_shards.data(), {static_cast<int64_t>(block_shards.size())}, long_cpu).clone();
    auto block_shards_device = block_shards_cpu.to(device);
    emit_block_shards_kernel<<<block_shards.size(), 1024, 0, stream>>>(
        folded.data_ptr<uint32_t>(), shard_meta.data_ptr<int64_t>(), analysis.data_ptr<int64_t>(),
        offsets.data_ptr<int64_t>(), block_shards_device.data_ptr<int64_t>(),
        block_shards.size(), output_storage.data_ptr<uint8_t>());
    C10_CUDA_KERNEL_LAUNCH_CHECK();
  }
  return output;
}
