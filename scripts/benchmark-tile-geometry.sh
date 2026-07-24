#!/usr/bin/env bash
# Serial single-trial screen of rectangular tile geometries.
set -euo pipefail

if [[ "$#" -lt 2 || "$#" -gt 3 ]]; then
  echo "usage: $0 BINARY OUTPUT [CORPUS]" >&2
  exit 1
fi

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
binary="$1"
output="$2"
corpus_dir="${3:-$repo_dir/artifacts/corpus-v2}"
mkdir -p "$(dirname -- "$output")"

temporary_dir="$(mktemp -d /tmp/fastvid-tile-geometry.XXXXXX)"
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
  $'64\t64'
  $'128\t64'
  $'128\t128'
  $'192\t192'
  $'256\t64'
  $'256\t128'
  $'256\t256'
  $'256\t384'
  $'256\t512'
  $'384\t256'
  $'512\t128'
  $'512\t256'
  $'512\t512'
  $'1024\t128'
  $'1024\t256'
)

first=1
for case_spec in "${cases[@]}"; do
  IFS=$'\t' read -r case_id input width height frame_rate frames quality threads gop \
    <<< "$case_spec"
  for geometry in "${geometries[@]}"; do
    IFS=$'\t' read -r tile_width tile_height <<< "$geometry"
    "$binary" benchmark-yuv422 \
      "$input" "$width" "$height" "$frame_rate" "$frames" \
      "$quality" "$threads" "$gop" "$tile_width" "$tile_height" > /dev/null
    result="$("$binary" benchmark-yuv422 \
      "$input" "$width" "$height" "$frame_rate" "$frames" \
      "$quality" "$threads" "$gop" "$tile_width" "$tile_height")"
    if [[ "$first" == 1 ]]; then
      printf 'case\t%s\n' "$(printf '%s\n' "$result" | head -n 1)" > "$output"
      first=0
    fi
    printf '%s\t%s\n' "$case_id" "$(printf '%s\n' "$result" | tail -n 1)" >> "$output"
  done
done

echo "results: $output"
