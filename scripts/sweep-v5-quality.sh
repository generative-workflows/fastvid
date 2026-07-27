#!/usr/bin/env bash
# Deterministic first-frame v5 rate-distortion sweep over the current corpus.
set -euo pipefail

if [[ "$#" -lt 3 ]]; then
  echo "usage: $0 FASTVID CORPUS_V3 OUTPUT.tsv [QUALITY ...]" >&2
  exit 2
fi

binary="$1"
corpus_dir="$2"
output="$3"
shift 3
qualities=("$@")
if [[ "${#qualities[@]}" -eq 0 ]]; then
  qualities=(80 85 90 95 100)
fi
repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="$repo_dir/corpus/manifest.json"
temporary_dir="$(mktemp -d /tmp/fastvid-quality-sweep.XXXXXX)"
trap 'rm -rf -- "$temporary_dir"' EXIT

[[ -x "$binary" ]] || { echo "binary is not executable: $binary" >&2; exit 1; }
for quality in "${qualities[@]}"; do
  [[ "$quality" =~ ^[0-9]+$ ]] && (( quality >= 1 && quality <= 100 )) || {
    echo "quality must be in 1..100: $quality" >&2
    exit 1
  }
done
mkdir -p "$(dirname -- "$output")"
printf 'sample\tsource\twidth\theight\tquality\tquant_step\traw_bytes\tencoded_bytes\tratio\txpsnr_y_db\txpsnr_u_db\txpsnr_v_db\txpsnr_min_db\tcompression_gt_15x\txpsnr_gt_50db\texact\n' > "$output"

while IFS=$'\t' read -r sample relative_path width height frame_rate source_id; do
  source="$corpus_dir/$relative_path"
  [[ -f "$source" ]] || { echo "missing corpus sample: $source" >&2; exit 1; }
  input="$temporary_dir/${sample}-yuv422p10le.yuv"
  ffmpeg -nostdin -hide_banner -loglevel error -y \
    -f rawvideo -pixel_format yuv422p -video_size "${width}x${height}" -framerate "$frame_rate" -i "$source" \
    -frames:v 1 -pix_fmt yuv422p10le -f rawvideo "$input"
  raw_bytes="$(stat -c %s "$input")"

  for quality in "${qualities[@]}"; do
    stream="$temporary_dir/${sample}-q${quality}.fvid"
    decoded="$temporary_dir/${sample}-q${quality}-decoded.yuv"
    "$binary" encode-yuv422p16le-parallel-full-tile \
      "$input" "$stream" "$width" "$height" "$frame_rate" 10 "$quality" 1 256 128
    "$binary" decode16 "$stream" "$decoded" 1
    encoded_bytes="$(stat -c %s "$stream")"
    ratio="$(awk -v raw="$raw_bytes" -v encoded="$encoded_bytes" 'BEGIN { printf "%.9f", raw / encoded }')"
    quant_step="$((1 + ((100 - quality) / 5) * 4))"

    stats="$temporary_dir/${sample}-q${quality}-xpsnr.log"
    ffmpeg -nostdin -hide_banner -loglevel error \
      -f rawvideo -pixel_format yuv422p10le -video_size "${width}x${height}" -framerate "$frame_rate" -i "$decoded" \
      -f rawvideo -pixel_format yuv422p10le -video_size "${width}x${height}" -framerate "$frame_rate" -i "$input" \
      -lavfi "xpsnr=stats_file=$stats" -f null -
    read -r xpsnr_y xpsnr_u xpsnr_v xpsnr_min < <(
      awk '
        /^XPSNR average,/ {
          for (i = 1; i <= NF; i++) {
            if ($i == "y:") y = $(i + 1)
            if ($i == "u:") u = $(i + 1)
            if ($i == "v:") v = $(i + 1)
            if ($i == "(minimum:") { minimum = $(i + 1); sub(/\)$/, "", minimum) }
          }
        }
        END { print y, u, v, minimum }
      ' "$stats"
    )
    [[ -n "$xpsnr_y" && -n "$xpsnr_min" ]] || { echo "failed to parse XPSNR" >&2; exit 1; }
    compression_pass="$(awk -v value="$ratio" 'BEGIN { print (value > 15.0 ? "true" : "false") }')"
    xpsnr_pass="$(awk -v value="$xpsnr_y" 'BEGIN { print (value == "inf" || value > 50.0) ? "true" : "false" }')"
    exact="false"
    [[ "$quality" -eq 100 ]] && cmp -s "$input" "$decoded" && exact="true"
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
      "$sample" "$source_id" "$width" "$height" "$quality" "$quant_step" \
      "$raw_bytes" "$encoded_bytes" "$ratio" "$xpsnr_y" "$xpsnr_u" "$xpsnr_v" \
      "$xpsnr_min" "$compression_pass" "$xpsnr_pass" "$exact" >> "$output"
  done
done < <(jq -r '
  .samples[] | select(.track == "codec")
  | [.id, .path, (.width | tostring), (.height | tostring), .frame_rate, .source]
  | @tsv
' "$manifest")

echo "quality sweep: $output"
