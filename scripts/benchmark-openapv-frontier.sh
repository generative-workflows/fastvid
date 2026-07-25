#!/usr/bin/env bash
# EXP-0073 matched native-10-bit Fastvid frontier/OpenAPV comparison.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
output="${1:-$repo_dir/artifacts/exp0073-openapv-frontier.tsv}"
openapv_build="${2:-/tmp/openapv-cmake-build}"
corpus_dir="${3:-$repo_dir/artifacts/corpus-v2}"
trials="${4:-5}"
manifest="${5:-$repo_dir/frontier.json}"

if (( trials < 3 )); then
  echo "at least three trials are required" >&2
  exit 1
fi

encoder="$openapv_build/bin/oapv_app_enc"
decoder="$openapv_build/bin/oapv_app_dec"
input="$corpus_dir/native/high-precision-motion-1280x720-24f-yuv422p10le.raw"
width=1280
height=720
frames=24
fps=24
bit_depth=10
raw_bytes="$(stat -c %s "$input")"
pixels=$((width * height * frames))

mapfile -t slots < <(jq -r '.slots[] | select(.state != "vacant") | .id' "$manifest")
declare -A binaries labels
for slot in "${slots[@]}"; do
  binary="$(jq -r --arg id "$slot" \
    '.slots[] | select(.id == $id) | .binary' "$manifest")"
  expected="$(jq -r --arg id "$slot" \
    '.slots[] | select(.id == $id) | .binary_sha256' "$manifest")"
  labels["$slot"]="$(jq -r --arg id "$slot" \
    '.slots[] | select(.id == $id) | .label' "$manifest")"
  if [[ "$binary" != /* ]]; then
    binary="$repo_dir/$binary"
  fi
  observed="$(sha256sum "$binary" | cut -d ' ' -f 1)"
  if [[ "$observed" != "$expected" ]]; then
    echo "Fastvid binary hash mismatch for $slot" >&2
    exit 1
  fi
  binaries["$slot"]="$binary"
done

for required in "$encoder" "$decoder" "$input"; do
  if [[ ! -e "$required" ]]; then
    echo "missing required file: $required" >&2
    exit 1
  fi
done

mkdir -p "$(dirname -- "$output")"
work_dir="$(mktemp -d /tmp/fastvid-openapv-frontier.XXXXXX)"
bitstream="$work_dir/openapv.apv"
decoded="$work_dir/openapv.yuv"
cleanup() {
  rm -f -- "$bitstream" "$decoded"
  rmdir -- "$work_dir"
}
trap cleanup EXIT

printf '%s\n' \
  $'codec\tslot\tlabel\tpreset\tcontrol\tthreads\ttrial\tframes\tbit_depth\traw_bytes\tencoded_bytes\tratio\tbits_per_luma_pixel\tencode_ms\tdecode_ms\tencode_mpps\tdecode_mpps\tencode_raw_mb_s\tdecode_raw_mb_s\tencoded_stream_mb_s\tencoded_stream_mbps\ty_psnr\tcb_psnr\tcr_psnr\ty_block_ssim\tmax_error' \
  > "$output"

run_fastvid() {
  local slot="$1"
  local quality="$2"
  local threads="$3"
  local trial="$4"
  local result
  result="$("${binaries[$slot]}" benchmark-yuv422p16le \
    "$input" "$width" "$height" "$fps/1" "$frames" "$bit_depth" \
    "$quality" "$threads" 1)"
  printf '%s\n' "$result" | awk -F $'\t' \
    -v slot="$slot" -v label="${labels[$slot]}" -v quality="$quality" \
    -v threads="$threads" -v trial="$trial" -v pixels="$pixels" '
      NR == 1 {
        for (field = 1; field <= NF; field++) column[$field] = field
        next
      }
      NR == 2 {
        encoded = $(column["encoded_bytes"])
        printf "fastvid\t%s\t%s\tfrontier\tq%s\t%s\t%s", \
          slot, label, quality, threads, trial
        split("frames bit_depth raw_bytes encoded_bytes ratio", prefix, " ")
        for (field = 1; field <= length(prefix); field++) {
          printf "\t%s", $(column[prefix[field]])
        }
        printf "\t%.6f", encoded * 8 / pixels
        split("encode_ms decode_ms encode_mpps decode_mpps encode_raw_mb_s decode_raw_mb_s encoded_stream_mb_s encoded_stream_mbps y_psnr cb_psnr cr_psnr y_block_ssim max_error", suffix, " ")
        for (field = 1; field <= length(suffix); field++) {
          printf "\t%s", $(column[suffix[field]])
        }
        printf "\n"
      }
    ' >> "$output"
}

for quality in 90 100; do
  for threads in 1 4; do
    for slot in "${slots[@]}"; do
      "${binaries[$slot]}" benchmark-yuv422p16le \
        "$input" "$width" "$height" "$fps/1" "$frames" "$bit_depth" \
        "$quality" "$threads" 1 > /dev/null
    done
    for trial in $(seq 1 "$trials"); do
      offset=$(( (trial - 1) % ${#slots[@]} ))
      for position in "${!slots[@]}"; do
        index=$(( (position + offset) % ${#slots[@]} ))
        run_fastvid "${slots[$index]}" "$quality" "$threads" "$trial"
      done
    done
  done
done

metrics_binary="${binaries[practical-compression]}"
for preset in medium fastest; do
  for qp in 0 20 21 22 23 24; do
    for threads in 1 4; do
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
        metrics="$("$metrics_binary" metrics-yuv422p16le \
          "$input" "$decoded" "$width" "$height" "$frames" "$bit_depth" |
          tail -n 1)"
        read -r _ _ y_psnr cb_psnr cr_psnr y_ssim max_error <<< "$metrics"
        read -r ratio bpp encode_mpps decode_mpps encode_raw decode_raw \
          stream_mb stream_mbps <<< "$(awk \
          -v raw="$raw_bytes" -v encoded="$encoded_bytes" -v px="$pixels" \
          -v ems="$encode_ms" -v dms="$decode_ms" \
          -v frames="$frames" -v fps="$fps" 'BEGIN {
            printf "%.6f %.6f %.3f %.3f %.3f %.3f %.6f %.6f",
              raw/encoded, encoded*8/px, px/(ems*1000), px/(dms*1000),
              raw/(ems*1000), raw/(dms*1000),
              encoded*fps/frames/1000000, encoded*fps/frames*8/1000000
          }')"
        printf 'openapv\texternal\tOpenAPV %s\t%s\tqp%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
          "$preset" "$preset" "$qp" "$threads" "$trial" "$frames" \
          "$bit_depth" "$raw_bytes" "$encoded_bytes" "$ratio" "$bpp" \
          "$encode_ms" "$decode_ms" "$encode_mpps" "$decode_mpps" \
          "$encode_raw" "$decode_raw" "$stream_mb" "$stream_mbps" \
          "$y_psnr" "$cb_psnr" "$cr_psnr" "$y_ssim" "$max_error" \
          >> "$output"
      done
    done
  done
done

echo "results: $output"
echo "openapv_encoder_sha256=$(sha256sum "$encoder" | cut -d ' ' -f 1)"
echo "openapv_decoder_sha256=$(sha256sum "$decoder" | cut -d ' ' -f 1)"
