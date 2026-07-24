#!/usr/bin/env bash
# EXP-0053 complete 8/10/12/16-bit order-0 residual model.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
corpus_dir="${1:-$repo_dir/artifacts/corpus-v2}"
output="${2:-$repo_dir/artifacts/exp0053-order0-model.tsv}"
manifest="$repo_dir/corpus/manifest.json"
high_bit_manifest="$repo_dir/corpus/high-bit-manifest.json"

cargo build --release --bin entropy_model --manifest-path "$repo_dir/Cargo.toml"
binary="$repo_dir/target/release/entropy_model"
mkdir -p "$(dirname -- "$output")"
first=1

append_result() {
  local result="$1"
  if [[ "$first" == 1 ]]; then
    printf '%s\n' "$result" | head -n 1 > "$output"
    first=0
  fi
  printf '%s\n' "$result" | tail -n +2 >> "$output"
}

while IFS=$'\t' read -r id kind path width height frames frame_rate; do
  if [[ "$kind" == video ]]; then
    gop=12
  else
    gop=1
  fi
  for quality in 60 75 90 95 100; do
    append_result "$("$binary" yuv422 "$id" "$corpus_dir/$path" "$width" "$height" \
      "$frame_rate" "$frames" "$quality" 1 "$gop")"
  done
done < <(
  jq -r '.samples[] | select(.track == "codec") |
    [.id, .kind, .path, .width, .height, .frames, .frame_rate] | @tsv' "$manifest"
)

while IFS=$'\t' read -r id kind path width height frames frame_rate pixel_format; do
  bit_depth="${pixel_format#yuv422p}"
  bit_depth="${bit_depth%le}"
  if [[ "$kind" == video ]]; then
    gop=12
  else
    gop=1
  fi
  for quality in 90 100; do
    append_result "$("$binary" yuv422p16le "$id" "$corpus_dir/$path" "$width" \
      "$height" "$frame_rate" "$frames" "$bit_depth" "$quality" 1 "$gop")"
  done
done < <(
  jq -r '.samples[] |
    [.id, .kind, .path, .width, .height, .frames, .frame_rate, .pixel_format] |
    @tsv' "$high_bit_manifest"
)

echo "results: $output"
python3 "$script_dir/summarize-order0-model.py" "$output"

