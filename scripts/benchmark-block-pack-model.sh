#!/usr/bin/env bash
# EXP-0085 charged 128-symbol fixed-width block model.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
corpus_dir="${1:-$repo_dir/artifacts/corpus-v2}"
output="${2:-$repo_dir/artifacts/exp0085-block-pack-model.tsv}"
manifest="$repo_dir/corpus/high-bit-manifest.json"

cargo build --release --bin block_pack_model --manifest-path "$repo_dir/Cargo.toml"
binary="$repo_dir/target/release/block_pack_model"

printf '%s\n' \
  $'sample\tframes\tbit_depth\tquality\ttiles\tpacked_tiles\tpacked_y_tiles\tpacked_cb_tiles\tpacked_cr_tiles\tsavings_y_bytes\tsavings_cb_bytes\tsavings_cr_bytes\tcurrent_payload_bytes\tpacked_payload_bytes\thybrid_payload_bytes\toverhead_bytes\tcurrent_stream_bytes\tpacked_stream_bytes\thybrid_stream_bytes\tpacked_delta\thybrid_delta\tsquared_error\tmax_error' \
  > "$output"

while IFS=$'\t' read -r id path width height frames pixel_format; do
  bit_depth="${pixel_format#yuv422p}"
  bit_depth="${bit_depth%le}"
  for quality in 90 100; do
    "$binary" "$id" "$corpus_dir/$path" "$width" "$height" "$frames" \
      "$bit_depth" "$quality" | tail -n 1 >> "$output"
  done
done < <(
  jq -r '.samples[] |
    [.id, .path, .width, .height, .frames, .pixel_format] | @tsv' "$manifest"
)

echo "results: $output"
