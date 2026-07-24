#!/usr/bin/env bash
# EXP-0047 fast exact screening on independent still frames.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
corpus_dir="${1:-$repo_dir/artifacts/corpus-v2}"
output="${2:-$repo_dir/artifacts/exp0047-predictor-stills-screening.tsv}"

cargo build --release --bin predictor_model --manifest-path "$repo_dir/Cargo.toml"
binary="$repo_dir/target/release/predictor_model"
manifest="$repo_dir/corpus/manifest.json"
high_bit_manifest="$repo_dir/corpus/high-bit-manifest.json"

printf '%s\n' \
  $'sample\tframe\tbit_depth\tquality\tgop\ttile\tplane\twidth\theight\tsamples\tcurrent_mode\toracle_mode\tcurrent_entropy\toracle_entropy\tcurrent_bytes\toracle_bytes\tcurrent_sse\toracle_sse\tcurrent_max_error\toracle_max_error\tpaeth_bytes\tpaeth_sse\taverage_bytes\taverage_sse\tclamp_bytes\tclamp_sse\thalf_bytes\thalf_sse\ttemporal_bytes\ttemporal_sse' \
  > "$output"

while IFS=$'\t' read -r id path width height frames frame_rate; do
  for quality in 60 75 90 95 100; do
    "$binary" yuv422 "$id" "$corpus_dir/$path" "$width" "$height" \
      "$frame_rate" "$frames" "$quality" 1 1 | tail -n +2 >> "$output"
  done
done < <(
  jq -r '.samples[] | select(.track == "codec" and .kind == "image") |
    [.id, .path, .width, .height, .frames, .frame_rate] | @tsv' "$manifest"
)

while IFS=$'\t' read -r id path width height frames frame_rate pixel_format; do
  bit_depth="${pixel_format#yuv422p}"
  bit_depth="${bit_depth%le}"
  for quality in 90 100; do
    "$binary" yuv422p16le "$id" "$corpus_dir/$path" "$width" "$height" \
      "$frame_rate" "$frames" "$bit_depth" "$quality" 1 1 \
      | tail -n +2 >> "$output"
  done
done < <(
  jq -r '.samples[] | select(.kind == "image") |
    [.id, .path, .width, .height, .frames, .frame_rate, .pixel_format] |
    @tsv' "$high_bit_manifest"
)

echo "results: $output"
python3 "$script_dir/summarize-predictor-model.py" "$output"
