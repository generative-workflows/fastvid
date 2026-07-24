#!/usr/bin/env bash
# Balanced warm-cache single-frame access comparison for native high-bit video.
set -euo pipefail

if [[ "$#" -lt 3 || "$#" -gt 7 ]]; then
  echo "usage: $0 BASELINE_BINARY CANDIDATE_BINARY OUTPUT [CORPUS] [QUALITIES] [BIT_DEPTHS] [TRIALS]" >&2
  exit 1
fi

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
baseline_binary="$1"
candidate_binary="$2"
output="$3"
corpus_dir="${4:-$repo_dir/artifacts/corpus-v2}"
qualities="${5:-90 100}"
bit_depths="${6:-10 16}"
trials="${7:-6}"
manifest="$repo_dir/corpus/high-bit-manifest.json"
targets="0,1,6,11,12,13,18,23"
mkdir -p "$(dirname -- "$output")"

if (( trials < 2 || trials % 2 != 0 )); then
  echo "TRIALS must be a positive even number for balanced execution order" >&2
  exit 1
fi

first=1
run_variant() {
  local variant="$1"
  local binary="$2"
  local trial="$3"
  local result
  result="$("$binary" benchmark-access-yuv422p16le \
    "$corpus_dir/$path" "$width" "$height" "$frame_rate" "$frames" \
    "$bit_depth" "$quality" 1 12 "$targets")"
  if [[ "$first" == 1 ]]; then
    printf 'variant\ttrial\t%s\n' "$(printf '%s\n' "$result" | head -n 1)" > "$output"
    first=0
  fi
  printf '%s\n' "$result" | tail -n +2 | while IFS= read -r row; do
    printf '%s\t%d\t%s\n' "$variant" "$trial" "$row" >> "$output"
  done
}

while IFS=$'\t' read -r path frames frame_rate width height bit_depth; do
  if [[ " $bit_depths " != *" $bit_depth "* ]]; then
    continue
  fi
  for quality in $qualities; do
    for trial in $(seq 1 "$trials"); do
      if (( trial % 2 == 1 )); then
        run_variant baseline "$baseline_binary" "$trial"
        run_variant candidate "$candidate_binary" "$trial"
      else
        run_variant candidate "$candidate_binary" "$trial"
        run_variant baseline "$baseline_binary" "$trial"
      fi
    done
  done
done < <(jq -r '
  .samples[]
  | select(.kind == "video")
  | [
      .path,
      (.frames | tostring),
      .frame_rate,
      (.width | tostring),
      (.height | tostring),
      (.pixel_format | capture("yuv422p(?<depth>[0-9]+)le").depth)
    ]
  | @tsv
' "$manifest")

echo "results: $output"
