#!/usr/bin/env bash
# EXP-0037 complete codec-track SSIM sampling matrix.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
corpus_dir="${1:-$repo_dir/artifacts/corpus-v2}"
output="${2:-$repo_dir/artifacts/exp0037-ssim-sampling.tsv}"

cargo build --release --bin ssim_sampling --manifest-path "$repo_dir/Cargo.toml"
binary="$repo_dir/target/release/ssim_sampling"
manifest="$repo_dir/corpus/manifest.json"

first=1
while IFS=$'\t' read -r id kind path width height frames frame_rate; do
  if [[ "$kind" == video ]]; then
    gop=12
  else
    gop=1
  fi
  for quality in 60 75 90 95 100; do
    result="$("$binary" "$corpus_dir/$path" "$width" "$height" "$frame_rate" \
      "$frames" "$quality" 1 "$gop")"
    if [[ "$first" == 1 ]]; then
      printf 'sample\tkind\t%s\n' "$(printf '%s\n' "$result" | head -n 1)" > "$output"
      first=0
    fi
    printf '%s\t%s\t%s\n' \
      "$id" "$kind" "$(printf '%s\n' "$result" | tail -n 1)" >> "$output"
  done
done < <(
  jq -r '.samples[] | select(.track == "codec") |
    [.id, .kind, .path, .width, .height, .frames, .frame_rate] | @tsv' "$manifest"
)

echo "results: $output"
python3 "$script_dir/summarize-ssim-sampling.py" "$output"
