#!/usr/bin/env bash
# Balanced repeated screen of the EXP-0064 default and two finalists.
set -euo pipefail

if [[ "$#" -lt 2 || "$#" -gt 4 ]]; then
  echo "usage: $0 BINARY OUTPUT [CORPUS] [TRIALS]" >&2
  exit 1
fi

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
binary="$1"
output="$2"
corpus_dir="${3:-$repo_dir/artifacts/corpus-v2}"
trials="${4:-6}"
if (( trials < 3 || trials % 3 != 0 )); then
  echo "TRIALS must be a positive multiple of three" >&2
  exit 1
fi
mkdir -p "$(dirname -- "$output")"

temporary_dir="$(mktemp -d /tmp/fastvid-tile-shortlist.XXXXXX)"
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
geometries=(
  $'256\t128'
  $'192\t192'
  $'256\t384'
)

first=1
run_geometry() {
  local tile_width="$1"
  local tile_height="$2"
  local trial="$3"
  local result
  result="$("$binary" benchmark-yuv422 \
    "$input" "$width" "$height" "$frame_rate" "$frames" \
    "$quality" "$threads" "$gop" "$tile_width" "$tile_height")"
  if [[ "$first" == 1 ]]; then
    printf 'trial\tcase\t%s\n' \
      "$(printf '%s\n' "$result" | head -n 1)" > "$output"
    first=0
  fi
  printf '%d\t%s\t%s\n' "$trial" "$case_id" \
    "$(printf '%s\n' "$result" | tail -n 1)" >> "$output"
}

for case_spec in "${cases[@]}"; do
  IFS=$'\t' read -r case_id input width height frame_rate frames quality threads gop \
    <<< "$case_spec"
  for geometry in "${geometries[@]}"; do
    IFS=$'\t' read -r tile_width tile_height <<< "$geometry"
    "$binary" benchmark-yuv422 \
      "$input" "$width" "$height" "$frame_rate" "$frames" \
      "$quality" "$threads" "$gop" "$tile_width" "$tile_height" > /dev/null
  done
  for trial in $(seq 1 "$trials"); do
    offset=$(( (trial - 1) % ${#geometries[@]} ))
    for position in "${!geometries[@]}"; do
      index=$(( (position + offset) % ${#geometries[@]} ))
      IFS=$'\t' read -r tile_width tile_height <<< "${geometries[$index]}"
      run_geometry "$tile_width" "$tile_height" "$trial"
    done
  done
done

echo "results: $output"
