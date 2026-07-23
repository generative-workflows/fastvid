#!/usr/bin/env bash
# Balanced preserved-binary version of the sub-minute feedback loop.
set -euo pipefail

if [[ "$#" -lt 3 || "$#" -gt 5 ]]; then
  echo "usage: $0 BASELINE_BINARY CANDIDATE_BINARY OUTPUT [CORPUS] [TRIALS]" >&2
  exit 1
fi

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
baseline_binary="$1"
candidate_binary="$2"
output="$3"
corpus_dir="${4:-$repo_dir/artifacts/corpus-v2}"
trials="${5:-6}"
mkdir -p "$(dirname -- "$output")"

if (( trials < 2 || trials % 2 != 0 )); then
  echo "TRIALS must be a positive even number for balanced execution order" >&2
  exit 1
fi

temporary_dir="$(mktemp -d /tmp/fastvid-ab-feedback.XXXXXX)"
ui_prefix="$temporary_dir/ui-4f.yuv"
cuts_prefix="$temporary_dir/cuts-4f.yuv"
cleanup() {
  rm -f -- "$ui_prefix" "$cuts_prefix"
  rmdir -- "$temporary_dir"
}
trap cleanup EXIT
dd if="$corpus_dir/videos/ui-dashboard-scroll-1280x720-24f.yuv" \
  of="$ui_prefix" bs=7372800 count=1 status=none
dd if="$corpus_dir/videos/procedural-scene-cuts-1920x1080-24f.yuv" \
  of="$cuts_prefix" bs=16588800 count=1 status=none

cases=(
  $'grid-4k\t'"$corpus_dir"$'/stills/resolution-grid-3840x2160.yuv\t3840\t2160\t24/1\t1\t100\t1\t1'
  $'camera-1080p\t'"$corpus_dir"$'/stills/camera-cholla-1920x1080.yuv\t1920\t1080\t24/1\t1\t90\t1\t1'
  $'ui-temporal-720p\t'"$ui_prefix"$'\t1280\t720\t24/1\t4\t90\t4\t12'
  $'cuts-temporal-1080p\t'"$cuts_prefix"$'\t1920\t1080\t24/1\t4\t90\t1\t12'
)

first=1
run_variant() {
  local variant="$1"
  local binary="$2"
  local trial="$3"
  local result
  result="$("$binary" benchmark-yuv422 \
    "$input" "$width" "$height" "$frame_rate" "$frames" \
    "$quality" "$threads" "$gop")"
  if [[ "$first" == 1 ]]; then
    printf 'variant\ttrial\tcase\t%s\n' \
      "$(printf '%s\n' "$result" | head -n 1)" > "$output"
    first=0
  fi
  printf '%s\t%d\t%s\t%s\n' \
    "$variant" "$trial" "$case_id" "$(printf '%s\n' "$result" | tail -n 1)" >> "$output"
}

for case_spec in "${cases[@]}"; do
  IFS=$'\t' read -r case_id input width height frame_rate frames quality threads gop \
    <<< "$case_spec"
  "$baseline_binary" benchmark-yuv422 \
    "$input" "$width" "$height" "$frame_rate" "$frames" \
    "$quality" "$threads" "$gop" > /dev/null
  "$candidate_binary" benchmark-yuv422 \
    "$input" "$width" "$height" "$frame_rate" "$frames" \
    "$quality" "$threads" "$gop" > /dev/null
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

echo "results: $output"
