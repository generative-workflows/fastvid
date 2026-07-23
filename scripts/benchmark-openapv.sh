#!/usr/bin/env bash
# Serial native-10-bit OpenAPV/Fastvid rate-quality and timing comparison.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
openapv_build="${1:-/tmp/openapv-cmake-build}"
corpus_dir="${2:-$repo_dir/artifacts/corpus-v2}"
output="${3:-$repo_dir/artifacts/openapv-comparison.tsv}"
trials="${4:-1}"
openapv_qps="${5:-0 8 16 20 24 28 32 40}"
openapv_presets="${6:-medium fastest}"
thread_counts="${7:-1 4}"
fastvid_qualities="${8:-90 100}"

encoder="$openapv_build/bin/oapv_app_enc"
decoder="$openapv_build/bin/oapv_app_dec"
fastvid="$repo_dir/target/release/fastvid"
input="$corpus_dir/native/high-precision-motion-1280x720-24f-yuv422p10le.raw"
width=1280
height=720
frames=24
fps=24
bit_depth=10

for required in "$encoder" "$decoder" "$fastvid" "$input"; do
  if [[ ! -e "$required" ]]; then
    echo "missing required file: $required" >&2
    exit 1
  fi
done
if (( trials < 1 )); then
  echo "trials must be at least 1" >&2
  exit 1
fi

mkdir -p "$(dirname -- "$output")"
work_dir="$(mktemp -d /tmp/fastvid-openapv.XXXXXX)"
trap 'rm -rf -- "$work_dir"' EXIT

header=$'codec\tpreset\tcontrol\tthreads\ttrial\tframes\tbit_depth\traw_bytes\tencoded_bytes\tratio\tbits_per_luma_pixel\tencode_ms\tdecode_ms\tencode_mpps\tdecode_mpps\tencode_raw_mb_s\tdecode_raw_mb_s\tencoded_stream_mb_s\tencoded_stream_mbps\ty_psnr\tcb_psnr\tcr_psnr\ty_block_ssim\tmax_error'
printf '%s\n' "$header" > "$output"
raw_bytes="$(stat -c %s "$input")"
pixels=$((width * height * frames))

# Fastvid's benchmark includes a warm-up-equivalent complete invocation per
# cell. Runs remain serial so the host never executes competing codec work.
for quality in $fastvid_qualities; do
  for threads in $thread_counts; do
    "$fastvid" benchmark-yuv422p16le \
      "$input" "$width" "$height" "$fps/1" "$frames" "$bit_depth" \
      "$quality" "$threads" 1 > /dev/null
    for trial in $(seq 1 "$trials"); do
      row="$("$fastvid" benchmark-yuv422p16le \
        "$input" "$width" "$height" "$fps/1" "$frames" "$bit_depth" \
        "$quality" "$threads" 1 | tail -n 1)"
      IFS=$'\t' read -r _ fv_frames fv_depth _ _ _ fv_raw fv_encoded \
        fv_ratio fv_encode_ms fv_decode_ms fv_encode_mpps fv_decode_mpps \
        fv_encode_raw fv_decode_raw fv_stream_mb fv_stream_mbps fv_y_psnr \
        fv_cb_psnr fv_cr_psnr fv_ssim fv_max_error <<< "$row"
      fv_bpp="$(awk -v enc="$fv_encoded" -v px="$pixels" \
        'BEGIN { printf "%.6f", enc * 8 / px }')"
      printf 'fastvid\tcurrent\tq%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$quality" "$threads" "$trial" "$fv_frames" "$fv_depth" "$fv_raw" \
        "$fv_encoded" "$fv_ratio" "$fv_bpp" "$fv_encode_ms" "$fv_decode_ms" \
        "$fv_encode_mpps" "$fv_decode_mpps" "$fv_encode_raw" "$fv_decode_raw" \
        "$fv_stream_mb" "$fv_stream_mbps" "$fv_y_psnr" "$fv_cb_psnr" \
        "$fv_cr_psnr" "$fv_ssim" "$fv_max_error" >> "$output"
    done
  done
done

for preset in $openapv_presets; do
  for qp in $openapv_qps; do
    for threads in $thread_counts; do
      bitstream="$work_dir/openapv.apv"
      decoded="$work_dir/openapv.yuv"
      common_args=(
        -i "$input" -w "$width" -h "$height" -z "$fps"
        -d "$bit_depth" --input-csp 2 --profile 422-10
        --preset "$preset" -q "$qp" -m "$threads" --max-au "$frames"
        --tile-w 256 --tile-h 128 -v 2
      )

      "$encoder" "${common_args[@]}" -o "$bitstream" > /dev/null
      "$decoder" -i "$bitstream" --max-au "$frames" -m "$threads" \
        -o "$decoded" -v 2 > /dev/null

      for trial in $(seq 1 "$trials"); do
        encode_log="$("$encoder" "${common_args[@]}" -o "$bitstream" 2>&1)"
        decode_log="$("$decoder" -i "$bitstream" --max-au "$frames" \
          -m "$threads" -o "$decoded" -v 2 2>&1)"
        encode_ms="$(printf '%s\n' "$encode_log" |
          awk -F'= ' '/Total encoding time/{split($2,a," "); print a[1]}')"
        decode_ms="$(printf '%s\n' "$decode_log" |
          awk -F'= ' '/Total decoding time/{split($2,a," "); print a[1]}')"
        if [[ -z "$encode_ms" || -z "$decode_ms" ]]; then
          echo "failed to parse OpenAPV codec time" >&2
          exit 1
        fi

        encoded_bytes="$(stat -c %s "$bitstream")"
        metrics="$("$fastvid" metrics-yuv422p16le \
          "$input" "$decoded" "$width" "$height" "$frames" "$bit_depth" |
          tail -n 1)"
        read -r _ _ y_psnr cb_psnr cr_psnr y_ssim max_error <<< "$metrics"

        read -r ratio bpp encode_mpps decode_mpps encode_raw decode_raw \
          stream_mb stream_mbps <<< "$(awk -v raw="$raw_bytes" -v enc="$encoded_bytes" \
          -v px="$pixels" -v ems="$encode_ms" -v dms="$decode_ms" \
          -v frames="$frames" -v fps="$fps" 'BEGIN {
            printf "%.6f %.6f %.3f %.3f %.3f %.3f %.6f %.6f",
              raw/enc, enc*8/px, px/(ems*1000), px/(dms*1000),
              raw/(ems*1000), raw/(dms*1000),
              enc*fps/frames/1000000, enc*fps/frames*8/1000000
          }')"
        printf 'openapv\t%s\tqp%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
          "$preset" "$qp" "$threads" "$trial" "$frames" "$bit_depth" \
          "$raw_bytes" "$encoded_bytes" "$ratio" "$bpp" "$encode_ms" "$decode_ms" \
          "$encode_mpps" "$decode_mpps" "$encode_raw" "$decode_raw" \
          "$stream_mb" "$stream_mbps" \
          "$y_psnr" "$cb_psnr" "$cr_psnr" "$y_ssim" "$max_error" >> "$output"
      done
    done
  done
done

echo "results: $output"
