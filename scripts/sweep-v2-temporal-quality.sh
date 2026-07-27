#!/usr/bin/env bash
# Exact Rust v2 GOP rate/minimum-frame-XPSNR feasibility control.
set -euo pipefail

if [[ "$#" -lt 3 ]]; then
  echo "usage: $0 V2_TEMPORAL_SWEEP CORPUS OUTPUT.tsv [QUALITY ...]" >&2
  exit 2
fi

binary="$1"
corpus_dir="$2"
output="$3"
shift 3
qualities=("$@")
if [[ "${#qualities[@]}" -eq 0 ]]; then
  qualities=(95 100)
fi
repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="$repo_dir/corpus/manifest.json"
temporary_dir="$(mktemp -d /tmp/fastvid-v2-temporal.XXXXXX)"
trap 'rm -rf -- "$temporary_dir"' EXIT

[[ -x "$binary" ]] || { echo "binary is not executable: $binary" >&2; exit 1; }
mkdir -p "$(dirname -- "$output")"
if [[ ! -s "$output" ]]; then
  printf 'sample\tsource\twidth\theight\tframes\tframe\tquality\tgop\tkeyframe\traw_bytes\tencoded_bytes\tratio\txpsnr_y_db\txpsnr_u_db\txpsnr_v_db\txpsnr_frame_min_db\txpsnr_y_gt_50db\texact\n' > "$output"
fi

while IFS=$'\t' read -r sample relative_path width height frames frame_rate source_id; do
  source="$corpus_dir/$relative_path"
  [[ -f "$source" ]] || { echo "missing corpus sample: $source" >&2; exit 1; }
  input="$temporary_dir/input-yuv422p10le.yuv"
  ffmpeg -nostdin -hide_banner -loglevel error -y \
    -f rawvideo -pixel_format yuv422p -video_size "${width}x${height}" -framerate "$frame_rate" -i "$source" \
    -frames:v "$frames" -pix_fmt yuv422p10le -f rawvideo "$input"
  if [[ "$frames" -eq 1 ]]; then
    gop=1
  else
    gop=12
  fi

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

    decoded="$temporary_dir/decoded.yuv"
    sizes="$temporary_dir/sizes.tsv"
    stats="$temporary_dir/xpsnr.log"
    metrics="$temporary_dir/metrics.tsv"
    "$binary" "$sample" "$input" "$decoded" "$width" "$height" "$frame_rate" \
      10 "$quality" 1 256 128 "$gop" > "$sizes"
    ffmpeg -nostdin -hide_banner -loglevel error \
      -f rawvideo -pixel_format yuv422p10le -video_size "${width}x${height}" -framerate "$frame_rate" -i "$decoded" \
      -f rawvideo -pixel_format yuv422p10le -video_size "${width}x${height}" -framerate "$frame_rate" -i "$input" \
      -lavfi "xpsnr=stats_file=$stats" -frames:v "$frames" -f null -
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
    [[ "$(($(wc -l < "$sizes") - 1))" -eq "$frames" ]] || {
      echo "wrong size-row count for $sample q$quality" >&2
      exit 1
    }
    [[ "$(wc -l < "$metrics")" -eq "$frames" ]] || {
      echo "failed to parse per-frame XPSNR for $sample q$quality" >&2
      exit 1
    }
    paste <(awk 'NR > 1' "$sizes") "$metrics" |
      while IFS=$'\t' read -r row_sample frame row_frames row_quality row_gop keyframe \
        raw_bytes encoded_bytes exact metric_frame xpsnr_y xpsnr_u xpsnr_v xpsnr_min; do
        [[ "$frame" -eq "$metric_frame" ]] || {
          echo "metric frame mismatch for $sample q$quality" >&2
          exit 1
        }
        ratio="$(awk -v raw="$raw_bytes" -v encoded="$encoded_bytes" 'BEGIN { printf "%.9f", raw / encoded }')"
        xpsnr_pass="$(awk -v value="$xpsnr_y" 'BEGIN { print (value == "inf" || value > 50.0) ? "true" : "false" }')"
        printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
          "$row_sample" "$source_id" "$width" "$height" "$row_frames" "$frame" \
          "$row_quality" "$row_gop" "$keyframe" "$raw_bytes" "$encoded_bytes" "$ratio" \
          "$xpsnr_y" "$xpsnr_u" "$xpsnr_v" "$xpsnr_min" "$xpsnr_pass" "$exact" >> "$output"
      done
  done
done < <(jq -r '
  .samples[] | select(.track == "codec")
  | [.id, .path, (.width | tostring), (.height | tostring), (.frames | tostring), .frame_rate, .source]
  | @tsv
' "$manifest")

echo "v2 temporal quality sweep: $output"
