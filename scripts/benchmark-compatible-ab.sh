#!/usr/bin/env bash
# Alternating A/B benchmark across binaries with compatible named columns.
set -euo pipefail

if [[ "$#" -lt 3 || "$#" -gt 5 ]]; then
  echo "usage: $0 OUTPUT BASELINE CANDIDATE [CORPUS] [TRIALS]" >&2
  exit 1
fi

output="$1"
baseline="$2"
candidate="$3"
corpus="${4:-artifacts/corpus-v2/videos/noisy-camera-fourpeople-1920x1080-24f.yuv}"
trials="${5:-6}"

if [[ ! -x "$baseline" || ! -x "$candidate" ]]; then
  echo "baseline and candidate must be executable" >&2
  exit 1
fi

mkdir -p "$(dirname -- "$output")"
printf '%s\n' \
  $'variant\ttrial\tencoded_bytes\tratio\tencode_mpps\tdecode_mpps\tencode_raw_mb_s\tdecode_raw_mb_s\tencoded_stream_mbps\ty_psnr\tcb_psnr\tcr_psnr\ty_block_ssim\tmax_error\tzero_run_tiles\trice_tiles\tspatial_tiles\ttemporal_tiles' \
  > "$output"

extract_row() {
  local variant="$1"
  local trial="$2"
  awk -F $'\t' -v variant="$variant" -v trial="$trial" '
    NR == 1 {
      for (field = 1; field <= NF; field++) {
        column[$field] = field
      }
      next
    }
    NR == 2 {
      printf "%s\t%d", variant, trial
      split("encoded_bytes ratio encode_mpps decode_mpps encode_raw_mb_s decode_raw_mb_s encoded_stream_mbps y_psnr cb_psnr cr_psnr y_block_ssim max_error zero_run_tiles rice_tiles spatial_tiles temporal_tiles", names, " ")
      for (field = 1; field <= length(names); field++) {
        printf "\t%s", $(column[names[field]])
      }
      printf "\n"
    }
  '
}

run_variant() {
  local variant="$1"
  local trial="$2"
  local binary="$3"
  "$binary" benchmark-yuv422 "$corpus" 1920 1080 24/1 24 90 1 1 |
    extract_row "$variant" "$trial" >> "$output"
}

"$candidate" benchmark-yuv422 "$corpus" 1920 1080 24/1 24 90 1 1 > /dev/null
for trial in $(seq 1 "$trials"); do
  if (( trial % 2 == 1 )); then
    run_variant baseline "$trial" "$baseline"
    run_variant candidate "$trial" "$candidate"
  else
    run_variant candidate "$trial" "$candidate"
    run_variant baseline "$trial" "$baseline"
  fi
done

echo "results: $output"
