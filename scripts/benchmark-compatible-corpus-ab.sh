#!/usr/bin/env bash
# Alternating two-binary A/B over the standard fast-feedback corpus.
set -euo pipefail

if [[ "$#" -lt 4 || "$#" -gt 5 ]]; then
  echo "usage: $0 OUTPUT BASELINE CANDIDATE CORPUS_DIR [TRIALS]" >&2
  exit 1
fi

output="$1"
baseline="$2"
candidate="$3"
corpus_dir="$4"
trials="${5:-6}"

if [[ ! -x "$baseline" || ! -x "$candidate" ]]; then
  echo "baseline and candidate must be executable" >&2
  exit 1
fi

temporary_dir="$(mktemp -d /tmp/fastvid-compatible-corpus.XXXXXX)"
ui_input="$temporary_dir/ui-4f.yuv"
cuts_input="$temporary_dir/cuts-4f.yuv"
cleanup() {
  rm -f -- "$ui_input" "$cuts_input"
  rmdir -- "$temporary_dir"
}
trap cleanup EXIT
dd if="$corpus_dir/videos/ui-dashboard-scroll-1280x720-24f.yuv" \
  of="$ui_input" bs=7372800 count=1 status=none
dd if="$corpus_dir/videos/procedural-scene-cuts-1920x1080-24f.yuv" \
  of="$cuts_input" bs=16588800 count=1 status=none

cases=(
  $'grid-4k\t'"$corpus_dir"$'/stills/resolution-grid-3840x2160.yuv\t3840\t2160\t24/1\t1\t100\t1\t1'
  $'camera-1080p\t'"$corpus_dir"$'/stills/camera-cholla-1920x1080.yuv\t1920\t1080\t24/1\t1\t90\t1\t1'
  $'ui-temporal-720p\t'"$ui_input"$'\t1280\t720\t24/1\t4\t90\t4\t12'
  $'cuts-temporal-1080p\t'"$cuts_input"$'\t1920\t1080\t24/1\t4\t90\t1\t12'
)

mkdir -p "$(dirname -- "$output")"
printf '%s\n' \
  $'case\tvariant\ttrial\tencoded_bytes\tratio\tencode_mpps\tdecode_mpps\tencoded_stream_mbps\ty_psnr\tcb_psnr\tcr_psnr\ty_block_ssim\tmax_error\tzero_run_tiles\trice_tiles\tspatial_tiles\ttemporal_tiles' \
  > "$output"

extract_row() {
  local case_id="$1"
  local variant="$2"
  local trial="$3"
  awk -F $'\t' -v case_id="$case_id" -v variant="$variant" -v trial="$trial" '
    NR == 1 {
      for (field = 1; field <= NF; field++) {
        column[$field] = field
      }
      next
    }
    NR == 2 {
      printf "%s\t%s\t%d", case_id, variant, trial
      split("encoded_bytes ratio encode_mpps decode_mpps encoded_stream_mbps y_psnr cb_psnr cr_psnr y_block_ssim max_error zero_run_tiles rice_tiles spatial_tiles temporal_tiles", names, " ")
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
  "$binary" benchmark-yuv422 \
    "$input" "$width" "$height" "$frame_rate" "$frames" \
    "$quality" "$threads" "$gop" |
    extract_row "$case_id" "$variant" "$trial" >> "$output"
}

for case_spec in "${cases[@]}"; do
  IFS=$'\t' read -r case_id input width height frame_rate frames quality threads gop \
    <<< "$case_spec"
  "$baseline" benchmark-yuv422 \
    "$input" "$width" "$height" "$frame_rate" "$frames" \
    "$quality" "$threads" "$gop" > /dev/null
  "$candidate" benchmark-yuv422 \
    "$input" "$width" "$height" "$frame_rate" "$frames" \
    "$quality" "$threads" "$gop" > /dev/null
  for trial in $(seq 1 "$trials"); do
    if (( trial % 2 == 1 )); then
      run_variant baseline "$trial" "$baseline"
      run_variant candidate "$trial" "$candidate"
    else
      run_variant candidate "$trial" "$candidate"
      run_variant baseline "$trial" "$baseline"
    fi
  done
done

echo "results: $output"
