#!/usr/bin/env bash
# EXP-0086 sampled block-pack selector confidence audit.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
corpus_dir="${1:-$repo_dir/artifacts/corpus-v2}"
output="${2:-$repo_dir/artifacts/exp0086-block-pack-selector.tsv}"
manifest="$repo_dir/corpus/high-bit-manifest.json"

cargo build --release --bin block_pack_model --manifest-path "$repo_dir/Cargo.toml"
binary="$repo_dir/target/release/block_pack_model"
first=1

while IFS=$'\t' read -r id path width height frames pixel_format; do
  bit_depth="${pixel_format#yuv422p}"
  bit_depth="${bit_depth%le}"
  for quality in 90 100; do
    result="$("$binary" "$id" "$corpus_dir/$path" "$width" "$height" "$frames" \
      "$bit_depth" "$quality")"
    if [[ "$first" == 1 ]]; then
      printf '%s\n' "$(printf '%s\n' "$result" | head -n 1)" > "$output"
      first=0
    fi
    printf '%s\n' "$result" | tail -n +2 >> "$output"
  done
done < <(
  jq -r '.samples[] |
    [.id, .path, .width, .height, .frames, .pixel_format] | @tsv' "$manifest"
)

echo "results: $output"
