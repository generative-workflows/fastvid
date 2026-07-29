#include <torch/extension.h>

#include <algorithm>
#include <cstdint>
#include <cstring>
#include <limits>
#include <map>
#include <mutex>
#include <tuple>
#include <vector>

std::vector<torch::Tensor> fastvid_decode_v5_cuda(
    const torch::Tensor& encoded,
    torch::Tensor shard_meta,
    const torch::Tensor& tile_meta,
    const torch::Tensor& tile_parse_meta,
    bool parse_metadata,
    int64_t total_samples,
    int64_t y_samples,
    int64_t chroma_samples,
    int64_t width,
    int64_t height,
    int64_t step,
    int64_t max_sample,
    bool grayscale,
    bool wavefront);

torch::Tensor fastvid_encode_v5_cuda(
    std::vector<torch::Tensor> planes,
    int64_t layout,
    int64_t bit_depth,
    int64_t quality,
    int64_t fps_numerator,
    int64_t fps_denominator,
    int64_t tile_width,
    int64_t tile_height);

namespace {

constexpr int64_t kHeaderBytes = 32;
constexpr int64_t kDirectoryEntryBytes = 32;
constexpr int64_t kShardSymbols = 4096;
constexpr uint8_t kVersion = 8;
constexpr uint8_t kFormatAwareVersion = 7;
constexpr uint8_t kSixthScaleVersion = 6;
constexpr uint8_t kLegacyVersion = 5;
constexpr uint8_t kEntropyParallelShards = 19;
constexpr uint8_t kPredictFullTileClampGradient = 6;

int64_t quantization_step_v7(int64_t layout, int64_t bit_depth, int64_t quality) {
  if (layout == 0 && bit_depth == 8) {
    return 1;
  }
  int64_t denominator = 6;
  if (layout == 1 && bit_depth == 10) {
    denominator = 20;
  } else if (layout == 1 && bit_depth == 16) {
    denominator = 12;
  } else if (layout == 2 && bit_depth == 10) {
    denominator = 10;
  }
  const int64_t scale = int64_t{1} << (bit_depth - 8);
  return 1 + ((100 - quality) * scale + denominator - 1) / denominator;
}

int64_t quantization_step_v8(int64_t layout, int64_t bit_depth, int64_t quality) {
  if (layout == 1 && bit_depth == 8) return 1;
  int64_t denominator = 6;
  if ((layout == 0 && bit_depth >= 10) || (layout == 2 && bit_depth == 16)) denominator = 12;
  else if (layout == 1 && bit_depth == 10) denominator = 20;
  else if (layout == 1 && bit_depth == 16) denominator = 12;
  else if (layout == 2 && bit_depth == 10) denominator = 10;
  const int64_t scale = int64_t{1} << (bit_depth - 8);
  return 1 + ((100 - quality) * scale + denominator - 1) / denominator;
}

uint16_t read_u16(const uint8_t* bytes, int64_t size, int64_t offset) {
  TORCH_CHECK(offset >= 0 && offset + 2 <= size, "truncated little-endian u16");
  return static_cast<uint16_t>(bytes[offset]) |
      (static_cast<uint16_t>(bytes[offset + 1]) << 8);
}

uint32_t read_u32(const uint8_t* bytes, int64_t size, int64_t offset) {
  TORCH_CHECK(offset >= 0 && offset + 4 <= size, "truncated little-endian u32");
  return static_cast<uint32_t>(bytes[offset]) |
      (static_cast<uint32_t>(bytes[offset + 1]) << 8) |
      (static_cast<uint32_t>(bytes[offset + 2]) << 16) |
      (static_cast<uint32_t>(bytes[offset + 3]) << 24);
}

uint64_t read_u64(const uint8_t* bytes, int64_t size, int64_t offset) {
  const uint64_t low = read_u32(bytes, size, offset);
  const uint64_t high = read_u32(bytes, size, offset + 4);
  return low | (high << 32);
}

struct Tile {
  int64_t plane;
  int64_t x;
  int64_t y;
  int64_t width;
  int64_t height;
  int64_t payload_offset;
  int64_t payload_length;
};

struct DecodeGeometry {
  torch::Tensor tile_meta;
  torch::Tensor tile_parse_meta;
  int64_t total_samples;
  int64_t total_shards;
  int64_t y_samples;
  int64_t secondary_samples;
};

using GeometryKey = std::tuple<int64_t, int64_t, int64_t, int64_t, int64_t, int64_t>;
std::mutex geometry_cache_mutex;
std::map<GeometryKey, DecodeGeometry> geometry_cache;

std::vector<Tile> expected_tiles(
    int64_t width,
    int64_t height,
    int64_t tile_width,
    int64_t tile_height,
    int64_t layout) {
  std::vector<Tile> result;
  const int64_t plane_count = layout == 0 ? 1 : 3;
  for (int64_t plane = 0; plane < plane_count; ++plane) {
    const bool subsampled = layout == 1 && plane != 0;
    const int64_t plane_width = subsampled ? (width + 1) / 2 : width;
    const int64_t nominal_width = subsampled ? (tile_width + 1) / 2 : tile_width;
    for (int64_t y = 0; y < height; y += tile_height) {
      for (int64_t x = 0; x < plane_width; x += nominal_width) {
        result.push_back(Tile{
            plane,
            x,
            y,
            std::min(nominal_width, plane_width - x),
            std::min(tile_height, height - y),
            0,
            0});
      }
    }
  }
  return result;
}

}  // namespace

std::vector<torch::Tensor> decode_v5(torch::Tensor encoded, bool wavefront) {
  TORCH_CHECK(encoded.scalar_type() == torch::kUInt8, "encoded must have dtype uint8");
  TORCH_CHECK(encoded.dim() == 1, "encoded must be one-dimensional");
  TORCH_CHECK(torch::cuda::is_available(), "CUDA is required");

  const auto size = encoded.numel();
  TORCH_CHECK(size >= kHeaderBytes, "truncated Fastvid header");
  auto host = encoded.is_cuda()
      ? encoded.narrow(0, 0, kHeaderBytes).to(torch::kCPU).contiguous()
      : encoded.contiguous();
  const auto* bytes = host.data_ptr<uint8_t>();
  TORCH_CHECK(std::memcmp(bytes, "FVID", 4) == 0, "bad Fastvid magic");
  TORCH_CHECK(bytes[4] == kLegacyVersion || bytes[4] == kSixthScaleVersion ||
                  bytes[4] == kFormatAwareVersion || bytes[4] == kVersion,
              "CUDA decoder requires Fastvid v5, v6, v7, or v8");
  const bool sixth_scale_quantizer = bytes[4] == kSixthScaleVersion;
  const bool format_aware_quantizer = bytes[4] == kFormatAwareVersion;
  const bool full_quality_quantizer = bytes[4] == kVersion;
  TORCH_CHECK(bytes[5] <= 2, "unknown pixel layout");
  const int64_t layout = bytes[5];
  const bool grayscale = layout == 0;
  const int64_t quality = bytes[6];
  TORCH_CHECK(quality >= 1 && quality <= 100, "quality is out of range");
  const int64_t bit_depth = static_cast<int64_t>(bytes[7]) + 8;
  TORCH_CHECK(bit_depth == 8 || bit_depth == 10 || bit_depth == 12 || bit_depth == 16,
              "unsupported high-bit depth");
  const int64_t width = read_u32(bytes, size, 8);
  const int64_t height = read_u32(bytes, size, 12);
  const int64_t tile_width = read_u16(bytes, size, 16);
  const int64_t tile_height = read_u16(bytes, size, 18);
  TORCH_CHECK(width > 0 && height > 0 && tile_width > 0 && tile_height > 0,
              "zero dimension");
  TORCH_CHECK(read_u32(bytes, size, 20) != 0 && read_u32(bytes, size, 24) != 0,
              "zero frame-rate component");
  const int64_t tile_count = read_u32(bytes, size, 28);
  auto tiles = expected_tiles(width, height, tile_width, tile_height, layout);
  TORCH_CHECK(tile_count == static_cast<int64_t>(tiles.size()),
              "tile count does not match dimensions");
  TORCH_CHECK(tile_count <= (1 << 20), "too many tiles");
  const int64_t directory_end = kHeaderBytes + tile_count * kDirectoryEntryBytes;
  TORCH_CHECK(directory_end <= size, "truncated tile directory");

  if (encoded.is_cuda()) {
    auto device_encoded = encoded.contiguous();
    const GeometryKey key{
        device_encoded.get_device(), layout, width, height, tile_width, tile_height};
    DecodeGeometry geometry;
    {
      std::lock_guard<std::mutex> lock(geometry_cache_mutex);
      auto found = geometry_cache.find(key);
      if (found == geometry_cache.end()) {
        auto long_cpu = torch::TensorOptions().dtype(torch::kInt64).device(torch::kCPU);
        auto tile_meta_cpu = torch::empty({tile_count, 7}, long_cpu);
        auto tile_parse_meta_cpu = torch::empty({tile_count, 2}, long_cpu);
        auto* tile_meta = tile_meta_cpu.data_ptr<int64_t>();
        auto* tile_parse_meta = tile_parse_meta_cpu.data_ptr<int64_t>();
        const int64_t y_samples = width * height;
        const int64_t secondary_width = layout == 1 ? (width + 1) / 2 : width;
        const int64_t secondary_samples = secondary_width * height;
        const int64_t plane_bases[3] = {0, y_samples, y_samples + secondary_samples};
        const int64_t plane_widths[3] = {width, secondary_width, secondary_width};
        int64_t total_samples = 0;
        int64_t total_shards = 0;
        for (int64_t i = 0; i < tile_count; ++i) {
          const auto& tile = tiles[i];
          tile_meta[i * 7 + 0] = plane_bases[tile.plane];
          tile_meta[i * 7 + 1] = plane_widths[tile.plane];
          tile_meta[i * 7 + 2] = tile.x;
          tile_meta[i * 7 + 3] = tile.y;
          tile_meta[i * 7 + 4] = tile.width;
          tile_meta[i * 7 + 5] = tile.height;
          tile_meta[i * 7 + 6] = total_samples;
          tile_parse_meta[i * 2 + 0] = total_shards;
          tile_parse_meta[i * 2 + 1] = tile.plane;
          const int64_t samples = tile.width * tile.height;
          total_samples += samples;
          total_shards += (samples + kShardSymbols - 1) / kShardSymbols;
        }
        DecodeGeometry created{
            tile_meta_cpu.to(device_encoded.device()),
            tile_parse_meta_cpu.to(device_encoded.device()),
            total_samples,
            total_shards,
            y_samples,
            secondary_samples};
        found = geometry_cache.emplace(key, std::move(created)).first;
      }
      geometry = found->second;
    }
    auto shard_meta_cuda = torch::empty(
        {geometry.total_shards, 5}, device_encoded.options().dtype(torch::kInt64));
    const int64_t scale = int64_t{1} << (bit_depth - 8);
    const int64_t step = full_quality_quantizer
        ? quantization_step_v8(layout, bit_depth, quality)
        : format_aware_quantizer
            ? quantization_step_v7(layout, bit_depth, quality)
        : sixth_scale_quantizer
            ? 1 + ((100 - quality) * scale + 5) / 6
            : 1 + (((100 - quality) / 5) * scale);
    const int64_t max_sample = (int64_t{1} << bit_depth) - 1;
    return fastvid_decode_v5_cuda(
        device_encoded,
        shard_meta_cuda,
        geometry.tile_meta,
        geometry.tile_parse_meta,
        true,
        geometry.total_samples,
        geometry.y_samples,
        geometry.secondary_samples,
        width,
        height,
        step,
        max_sample,
        grayscale,
        wavefront);
  }

  int64_t next_payload = directory_end;
  int64_t total_shards = 0;
  int64_t total_samples = 0;
  for (int64_t i = 0; i < tile_count; ++i) {
    const int64_t start = kHeaderBytes + i * kDirectoryEntryBytes;
    TORCH_CHECK(bytes[start] == tiles[i].plane, "non-canonical tile plane");
    TORCH_CHECK(bytes[start + 1] == kEntropyParallelShards,
                "v5 tile requires bounded-shard entropy");
    TORCH_CHECK(bytes[start + 2] == kPredictFullTileClampGradient,
                "v5 tile requires full-tile clamp-gradient prediction");
    TORCH_CHECK(bytes[start + 3] == 0, "nonzero reserved directory byte");
    TORCH_CHECK(read_u32(bytes, size, start + 4) == tiles[i].x &&
                    read_u32(bytes, size, start + 8) == tiles[i].y &&
                    read_u32(bytes, size, start + 12) == tiles[i].width &&
                    read_u32(bytes, size, start + 16) == tiles[i].height,
                "non-canonical tile directory");
    const uint64_t offset_u64 = read_u64(bytes, size, start + 20);
    TORCH_CHECK(offset_u64 <= static_cast<uint64_t>(std::numeric_limits<int64_t>::max()),
                "payload offset is too large");
    tiles[i].payload_offset = static_cast<int64_t>(offset_u64);
    tiles[i].payload_length = read_u32(bytes, size, start + 28);
    TORCH_CHECK(tiles[i].payload_offset == next_payload, "payloads are not contiguous");
    TORCH_CHECK(tiles[i].payload_length <= size - next_payload, "truncated tile payload");
    next_payload += tiles[i].payload_length;
    const int64_t samples = tiles[i].width * tiles[i].height;
    TORCH_CHECK(samples > 0 && total_samples <= std::numeric_limits<int64_t>::max() - samples,
                "frame is too large");
    total_samples += samples;
    total_shards += (samples + kShardSymbols - 1) / kShardSymbols;
  }
  TORCH_CHECK(next_payload == size, "trailing stream bytes");

  auto long_cpu = torch::TensorOptions().dtype(torch::kInt64).device(torch::kCPU);
  auto shard_meta_cpu = torch::empty({total_shards, 5}, long_cpu);
  auto tile_meta_cpu = torch::empty({tile_count, 7}, long_cpu);
  auto* shard_meta = shard_meta_cpu.data_ptr<int64_t>();
  auto* tile_meta = tile_meta_cpu.data_ptr<int64_t>();
  const int64_t y_samples = width * height;
  const int64_t secondary_width = layout == 1 ? (width + 1) / 2 : width;
  const int64_t secondary_samples = secondary_width * height;
  const int64_t plane_bases[3] = {0, y_samples, y_samples + secondary_samples};
  const int64_t plane_widths[3] = {width, secondary_width, secondary_width};

  int64_t shard_index = 0;
  int64_t folded_base = 0;
  for (int64_t i = 0; i < tile_count; ++i) {
    const auto& tile = tiles[i];
    tile_meta[i * 7 + 0] = plane_bases[tile.plane];
    tile_meta[i * 7 + 1] = plane_widths[tile.plane];
    tile_meta[i * 7 + 2] = tile.x;
    tile_meta[i * 7 + 3] = tile.y;
    tile_meta[i * 7 + 4] = tile.width;
    tile_meta[i * 7 + 5] = tile.height;
    tile_meta[i * 7 + 6] = folded_base;
    int64_t cursor = tile.payload_offset;
    int64_t decoded = 0;
    const int64_t tile_end = cursor + tile.payload_length;
    while (decoded < tile.width * tile.height) {
      TORCH_CHECK(cursor + 3 <= tile_end, "truncated entropy shard header");
      const int64_t mode = bytes[cursor];
      const int64_t body_length = read_u16(bytes, size, cursor + 1);
      const int64_t body_offset = cursor + 3;
      TORCH_CHECK(mode <= 19, "unknown entropy shard mode");
      TORCH_CHECK(body_length <= tile_end - body_offset, "truncated entropy shard body");
      const int64_t sample_count = std::min(kShardSymbols, tile.width * tile.height - decoded);
      shard_meta[shard_index * 5 + 0] = mode;
      shard_meta[shard_index * 5 + 1] = body_offset;
      shard_meta[shard_index * 5 + 2] = body_length;
      shard_meta[shard_index * 5 + 3] = sample_count;
      shard_meta[shard_index * 5 + 4] = folded_base + decoded;
      ++shard_index;
      decoded += sample_count;
      cursor = body_offset + body_length;
    }
    TORCH_CHECK(cursor == tile_end, "trailing bounded-shard tile bytes");
    folded_base += tile.width * tile.height;
  }
  TORCH_CHECK(shard_index == total_shards && folded_base == total_samples,
              "internal metadata count mismatch");

  auto device_encoded = encoded.is_cuda() ? encoded.contiguous() : encoded.to(torch::kCUDA);
  auto shard_meta_cuda = shard_meta_cpu.to(device_encoded.device());
  auto tile_meta_cuda = tile_meta_cpu.to(device_encoded.device());
  const int64_t scale = int64_t{1} << (bit_depth - 8);
  const int64_t step = full_quality_quantizer
      ? quantization_step_v8(layout, bit_depth, quality)
      : format_aware_quantizer
          ? quantization_step_v7(layout, bit_depth, quality)
      : sixth_scale_quantizer
          ? 1 + ((100 - quality) * scale + 5) / 6
          : 1 + (((100 - quality) / 5) * scale);
  const int64_t max_sample = (int64_t{1} << bit_depth) - 1;
  return fastvid_decode_v5_cuda(
      device_encoded,
      shard_meta_cuda,
      tile_meta_cuda,
      torch::Tensor(),
      false,
      total_samples,
      y_samples,
      secondary_samples,
      width,
      height,
      step,
      max_sample,
      grayscale,
      wavefront);
}

PYBIND11_MODULE(TORCH_EXTENSION_NAME, module) {
  module.def(
      "decode_v5",
      &decode_v5,
      "Decode a Fastvid v5 frame on CUDA",
      pybind11::arg("encoded"),
      pybind11::arg("wavefront") = true);
  module.def(
      "encode_v5",
      &fastvid_encode_v5_cuda,
      "Encode CUDA uint16 planes to a Rust-compatible Fastvid v5 frame",
      pybind11::arg("planes"),
      pybind11::arg("layout"),
      pybind11::arg("bit_depth"),
      pybind11::arg("quality"),
      pybind11::arg("fps_numerator") = 24,
      pybind11::arg("fps_denominator") = 1,
      pybind11::arg("tile_width") = 256,
      pybind11::arg("tile_height") = 128);
}
