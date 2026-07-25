#!/usr/bin/env bash
# EXP-0081 dependency-free above-predictor size/error screen.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
corpus_dir="${1:-$repo_dir/artifacts/corpus-v2}"
output="${2:-$repo_dir/artifacts/exp0081-above-model.tsv}"
manifest="$repo_dir/corpus/high-bit-manifest.json"

cargo build --release --bin above_model --manifest-path "$repo_dir/Cargo.toml"
binary="$repo_dir/target/release/above_model"

printf '%s\n' \
  $'sample\tframes\tbit_depth\tquality\ttiles\tclamp_payload_bytes\tabove_payload_bytes\toverhead_bytes\tclamp_stream_bytes\tabove_stream_bytes\tstream_delta\tclamp_sse\tabove_sse\tclamp_max_error\tabove_max_error' \
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
