#!/usr/bin/env bash
# Deterministic all-frame v5 rate/XPSNR sweep over the current corpus.
set -euo pipefail

if [[ "$#" -lt 3 ]]; then
  echo "usage: $0 FASTVID CORPUS OUTPUT.tsv [QUALITY ...]" >&2
  exit 2
fi

binary="$1"
corpus_dir="$2"
output="$3"
shift 3
qualities=("$@")
if [[ "${#qualities[@]}" -eq 0 ]]; then
  qualities=(80 85 90)
fi
repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="$repo_dir/corpus/manifest.json"
temporary_dir="$(mktemp -d /tmp/fastvid-frame-quality.XXXXXX)"
trap 'rm -rf -- "$temporary_dir"' EXIT

[[ -x "$binary" ]] || { echo "binary is not executable: $binary" >&2; exit 1; }
mkdir -p "$(dirname -- "$output")"
if [[ ! -s "$output" ]]; then
  printf 'sample\tsource\twidth\theight\tframes\tframe\tquality\tquant_step\traw_bytes\tencoded_bytes\tratio\txpsnr_y_db\txpsnr_u_db\txpsnr_v_db\txpsnr_frame_min_db\tcompression_gt_15x\txpsnr_y_gt_50db\texact\n' > "$output"
fi

while IFS=$'\t' read -r sample relative_path width height frames frame_rate source_id; do
  source="$corpus_dir/$relative_path"
  [[ -f "$source" ]] || { echo "missing corpus sample: $source" >&2; exit 1; }
  # Reuse bounded scratch paths so a full-corpus sweep does not retain one
  # multi-frame conversion and five decoded sequences per sample.
  input="$temporary_dir/input-yuv422p10le.yuv"
  ffmpeg -nostdin -hide_banner -loglevel error -y \
    -f rawvideo -pixel_format yuv422p -video_size "${width}x${height}" -framerate "$frame_rate" -i "$source" \
    -frames:v "$frames" -pix_fmt yuv422p10le -f rawvideo "$input"
  frame_bytes=$((width * height * 4))

  for quality in "${qualities[@]}"; do
    existing="$(awk -F '\t' -v sample="$sample" -v quality="$quality" '
      NR > 1 && $1 == sample && $7 == quality { count++ }
      END { print count + 0 }
    ' "$output")"
    if [[ "$existing" -eq "$frames" ]]; then
      continue
    fi
    if [[ "$existing" -ne 0 ]]; then
      echo "partial existing cell: $sample q$quality has $existing/$frames frames" >&2
      exit 1
    fi
    decoded_sequence="$temporary_dir/decoded-sequence.yuv"
    stats="$temporary_dir/xpsnr.log"
    : > "$decoded_sequence"
    encoded_sizes=()
    exact_frames=()
    for ((frame = 0; frame < frames; frame++)); do
      source_frame="$temporary_dir/source-frame.yuv"
      stream="$temporary_dir/frame.fvid"
      decoded_frame="$temporary_dir/decoded-frame.yuv"
      dd if="$input" of="$source_frame" bs="$frame_bytes" skip="$frame" count=1 status=none
      "$binary" encode-yuv422p16le-parallel-full-tile \
        "$source_frame" "$stream" "$width" "$height" "$frame_rate" 10 "$quality" 1 256 128
      "$binary" decode16 "$stream" "$decoded_frame" 1
      encoded_sizes[$frame]="$(stat -c %s "$stream")"
      exact_frames[$frame]="false"
      if [[ "$quality" -eq 100 ]] && cmp -s "$source_frame" "$decoded_frame"; then
        exact_frames[$frame]="true"
      fi
      dd if="$decoded_frame" status=none >> "$decoded_sequence"
    done
    ffmpeg -nostdin -hide_banner -loglevel error \
      -f rawvideo -pixel_format yuv422p10le -video_size "${width}x${height}" -framerate "$frame_rate" -i "$decoded_sequence" \
      -f rawvideo -pixel_format yuv422p10le -video_size "${width}x${height}" -framerate "$frame_rate" -i "$input" \
      -lavfi "xpsnr=stats_file=$stats" -frames:v "$frames" -f null -
    metrics="$temporary_dir/per-frame.tsv"
    awk '
      /^n:/ {
        n = y = u = v = ""
        for (i = 1; i <= NF; i++) {
          if ($i == "n:") n = $(i + 1)
          if ($i == "y:") y = $(i + 1)
          if ($i == "u:") u = $(i + 1)
          if ($i == "v:") v = $(i + 1)
        }
        minimum = y
        if (u != "inf" && (minimum == "inf" || u + 0 < minimum + 0)) minimum = u
        if (v != "inf" && (minimum == "inf" || v + 0 < minimum + 0)) minimum = v
        print n - 1, y, u, v, minimum
      }
    ' OFS='\t' "$stats" > "$metrics"
    [[ "$(wc -l < "$metrics")" -eq "$frames" ]] || {
      echo "failed to parse per-frame XPSNR for $sample q$quality" >&2
      exit 1
    }
    quant_step="$((1 + ((100 - quality) / 5) * 4))"
    while IFS=$'\t' read -r frame xpsnr_y xpsnr_u xpsnr_v xpsnr_min; do
      encoded_bytes="${encoded_sizes[$frame]}"
      ratio="$(awk -v raw="$frame_bytes" -v encoded="$encoded_bytes" 'BEGIN { printf "%.9f", raw / encoded }')"
      compression_pass="$(awk -v value="$ratio" 'BEGIN { print (value > 15.0 ? "true" : "false") }')"
      xpsnr_pass="$(awk -v value="$xpsnr_y" 'BEGIN { print (value == "inf" || value > 50.0) ? "true" : "false" }')"
      printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$sample" "$source_id" "$width" "$height" "$frames" "$frame" "$quality" "$quant_step" \
        "$frame_bytes" "$encoded_bytes" "$ratio" "$xpsnr_y" "$xpsnr_u" "$xpsnr_v" "$xpsnr_min" \
        "$compression_pass" "$xpsnr_pass" "${exact_frames[$frame]}" >> "$output"
    done < "$metrics"
  done
done < <(jq -r '
  .samples[] | select(.track == "codec")
  | [.id, .path, (.width | tostring), (.height | tostring), (.frames | tostring), .frame_rate, .source]
  | @tsv
' "$manifest")

echo "full-frame quality sweep: $output"
