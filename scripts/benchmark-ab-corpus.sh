#!/usr/bin/env bash
# Alternating baseline/candidate corpus comparison; see EXP-0019.
set -euo pipefail

if [[ "$#" -lt 3 || ("$#" -gt 9 && "$#" -ne 13) ]]; then
  echo "usage: $0 BASELINE_BINARY CANDIDATE_BINARY OUTPUT [CORPUS] [QUALITIES] [THREADS] [GOP] [KIND] [TRIALS [BASELINE_TILE_WIDTH BASELINE_TILE_HEIGHT CANDIDATE_TILE_WIDTH CANDIDATE_TILE_HEIGHT]]" >&2
  exit 1
fi

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
baseline_binary="$1"
candidate_binary="$2"
output="$3"
corpus_dir="${4:-$repo_dir/artifacts/corpus-v2}"
qualities="${5:-90 100}"
thread_counts="${6:-1 4}"
gop="${7:-1}"
kind="${8:-image}"
trials="${9:-6}"
baseline_geometry=()
candidate_geometry=()
if [[ "$#" -eq 13 ]]; then
  baseline_geometry=("${10}" "${11}")
  candidate_geometry=("${12}" "${13}")
fi
manifest="$repo_dir/corpus/manifest.json"
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
  local geometry=()
  local result
  if [[ "$variant" == "baseline" ]]; then
    geometry=("${baseline_geometry[@]}")
  else
    geometry=("${candidate_geometry[@]}")
  fi
  result="$("$binary" benchmark-yuv422 \
    "$corpus_dir/$path" "$width" "$height" "$frame_rate" "$frames" \
    "$quality" "$threads" "$gop" "${geometry[@]}")"
  if [[ "$first" == 1 ]]; then
    printf 'variant\ttrial\t%s\n' "$(printf '%s\n' "$result" | head -n 1)" > "$output"
    first=0
  fi
  printf '%s\t%d\t%s\n' \
    "$variant" "$trial" "$(printf '%s\n' "$result" | tail -n 1)" >> "$output"
}

for quality in $qualities; do
  for threads in $thread_counts; do
    while IFS=$'\t' read -r path frames frame_rate width height; do
      "$baseline_binary" benchmark-yuv422 \
        "$corpus_dir/$path" "$width" "$height" "$frame_rate" "$frames" \
        "$quality" "$threads" "$gop" "${baseline_geometry[@]}" > /dev/null
      "$candidate_binary" benchmark-yuv422 \
        "$corpus_dir/$path" "$width" "$height" "$frame_rate" "$frames" \
        "$quality" "$threads" "$gop" "${candidate_geometry[@]}" > /dev/null
      for trial in $(seq 1 "$trials"); do
        if (( trial % 2 == 1 )); then
          run_variant baseline "$baseline_binary" "$trial"
          run_variant candidate "$candidate_binary" "$trial"
        else
          run_variant candidate "$candidate_binary" "$trial"
          run_variant baseline "$baseline_binary" "$trial"
        fi
      done
    done < <(jq -r --arg kind "$kind" '
      .samples[]
      | select(.track == "codec" and .benchmark != false)
      | select($kind == "all" or .kind == $kind)
      | [.path, (.frames | tostring), .frame_rate, (.width | tostring), (.height | tostring)]
      | @tsv
    ' "$manifest")
  done
done

echo "results: $output"
