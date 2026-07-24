#!/usr/bin/env bash
# Balanced multi-version benchmark for the machine-readable frontier.
set -euo pipefail

if [[ "$#" -lt 1 || "$#" -gt 4 ]]; then
  echo "usage: $0 OUTPUT [CORPUS] [MANIFEST] [TRIALS]" >&2
  exit 1
fi

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
output="$1"
corpus_dir="${2:-$repo_dir/artifacts/corpus-v2}"
manifest="${3:-$repo_dir/frontier.json}"
trials="${4:-$(jq -r '.protocol.trials' "$manifest")}"
mkdir -p "$(dirname -- "$output")"

mapfile -t slots < <(jq -r '.slots[] | select(.state != "vacant") | .id' "$manifest")
if (( ${#slots[@]} < 2 || trials < ${#slots[@]} || trials % ${#slots[@]} != 0 )); then
  echo "TRIALS must be a positive multiple of the active frontier size" >&2
  exit 1
fi

declare -A binaries labels
for slot in "${slots[@]}"; do
  binary="$(jq -r --arg id "$slot" '.slots[] | select(.id == $id) | .binary' "$manifest")"
  expected="$(jq -r --arg id "$slot" '.slots[] | select(.id == $id) | .binary_sha256' "$manifest")"
  label="$(jq -r --arg id "$slot" '.slots[] | select(.id == $id) | .label' "$manifest")"
  if [[ "$binary" != /* ]]; then
    binary="$repo_dir/$binary"
  fi
  if [[ ! -x "$binary" ]]; then
    echo "frontier binary is missing or not executable: $binary" >&2
    exit 1
  fi
  observed="$(sha256sum "$binary" | cut -d ' ' -f 1)"
  if [[ "$observed" != "$expected" ]]; then
    echo "frontier binary hash mismatch for $slot: $observed != $expected" >&2
    exit 1
  fi
  binaries["$slot"]="$binary"
  labels["$slot"]="$label"
done

temporary_dir="$(mktemp -d /tmp/fastvid-frontier.XXXXXX)"
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
run_slot() {
  local slot="$1"
  local trial="$2"
  local result
  result="$("${binaries[$slot]}" benchmark-yuv422 \
    "$input" "$width" "$height" "$frame_rate" "$frames" \
    "$quality" "$threads" "$gop")"
  if [[ "$first" == 1 ]]; then
    printf 'slot\tlabel\ttrial\tcase\t%s\n' \
      "$(printf '%s\n' "$result" | head -n 1)" > "$output"
    first=0
  fi
  printf '%s\t%s\t%d\t%s\t%s\n' \
    "$slot" "${labels[$slot]}" "$trial" "$case_id" \
    "$(printf '%s\n' "$result" | tail -n 1)" >> "$output"
}

for case_spec in "${cases[@]}"; do
  IFS=$'\t' read -r case_id input width height frame_rate frames quality threads gop \
    <<< "$case_spec"
  for slot in "${slots[@]}"; do
    "${binaries[$slot]}" benchmark-yuv422 \
      "$input" "$width" "$height" "$frame_rate" "$frames" \
      "$quality" "$threads" "$gop" > /dev/null
  done
  for trial in $(seq 1 "$trials"); do
    offset=$(( (trial - 1) % ${#slots[@]} ))
    for position in "${!slots[@]}"; do
      index=$(( (position + offset) % ${#slots[@]} ))
      run_slot "${slots[$index]}" "$trial"
    done
  done
done

echo "results: $output"
