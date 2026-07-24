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

printf '%s\n' \
  $'slot\tlabel\ttrial\tcase\tinput\tframes\tquality\tthreads\tgop\traw_bytes\tencoded_bytes\tratio\tencode_ms\tdecode_ms\tencode_mpps\tdecode_mpps\tencode_raw_mb_s\tdecode_raw_mb_s\tencoded_stream_mb_s\tencoded_stream_mbps\ty_psnr\tcb_psnr\tcr_psnr\ty_block_ssim\tmax_error\tzero_run_tiles\trice_tiles\tspatial_tiles\ttemporal_tiles' \
  > "$output"

run_slot() {
  local slot="$1"
  local trial="$2"
  local result
  result="$("${binaries[$slot]}" benchmark-yuv422 \
    "$input" "$width" "$height" "$frame_rate" "$frames" \
    "$quality" "$threads" "$gop")"
  printf '%s\n' "$result" | awk -F $'\t' \
    -v slot="$slot" -v label="${labels[$slot]}" \
    -v trial="$trial" -v case_id="$case_id" '
      NR == 1 {
        for (field = 1; field <= NF; field++) {
          column[$field] = field
        }
        next
      }
      NR == 2 {
        printf "%s\t%s\t%d\t%s", slot, label, trial, case_id
        split("input frames quality threads gop raw_bytes encoded_bytes ratio encode_ms decode_ms encode_mpps decode_mpps encode_raw_mb_s decode_raw_mb_s encoded_stream_mb_s encoded_stream_mbps y_psnr cb_psnr cr_psnr y_block_ssim max_error zero_run_tiles rice_tiles spatial_tiles temporal_tiles", names, " ")
        for (field = 1; field <= length(names); field++) {
          printf "\t%s", $(column[names[field]])
        }
        printf "\n"
      }
    ' >> "$output"
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
