#!/usr/bin/env bash
# Native-format FFmpeg XPSNR for version-5 decoded sequences.
set -euo pipefail

if [[ "$#" -ne 4 ]]; then
  echo "usage: $0 BINARY CORPUS_DIR OUTPUT QUALITIES" >&2
  exit 2
fi

binary="$1"
corpus_dir="$2"
output="$3"
qualities="$4"
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
manifest="$repo_dir/corpus/high-bit-manifest.json"
temporary_dir="$(mktemp -d /tmp/fastvid-xpsnr.XXXXXX)"
trap 'rm -rf -- "$temporary_dir"' EXIT

if [[ ! -x "$binary" ]]; then
  echo "binary is not executable: $binary" >&2
  exit 1
fi
if ! ffmpeg -hide_banner -filters 2>/dev/null | grep -q ' xpsnr '; then
  echo "FFmpeg xpsnr filter is unavailable" >&2
  exit 1
fi
(
  cd "$corpus_dir"
  sha256sum --check "$repo_dir/corpus/high-bit-checksums.sha256"
) >/dev/null
mkdir -p "$(dirname -- "$output")"
printf 'sample\tbit_depth\tquality\tframes\tpixel_format\txpsnr_y_db\txpsnr_u_db\txpsnr_v_db\txpsnr_min_db\n' > "$output"

for quality in $qualities; do
  while IFS=$'\t' read -r id path frames frame_rate width height bit_depth pixel_format; do
    input="$corpus_dir/$path"
    frame_bytes=$((width * height * 4))

    stats="$temporary_dir/xpsnr.log"
    produce_decoded() {
      for ((frame = 0; frame < frames; frame++)); do
        dd if="$input" of="$temporary_dir/source.raw" bs="$frame_bytes" \
          skip="$frame" count=1 status=none
        "$binary" encode-yuv422p16le-parallel-full-tile \
          "$temporary_dir/source.raw" "$temporary_dir/frame.fvid" \
          "$width" "$height" "$frame_rate" "$bit_depth" "$quality" 1 256 128
        "$binary" decode16 \
          "$temporary_dir/frame.fvid" "$temporary_dir/decoded.raw" 1
        dd if="$temporary_dir/decoded.raw" status=none
      done
    }
    produce_decoded |
      ffmpeg -nostdin -hide_banner -loglevel error \
        -f rawvideo -pixel_format "$pixel_format" -video_size "${width}x${height}" \
        -framerate "$frame_rate" -i pipe:0 \
        -f rawvideo -pixel_format "$pixel_format" -video_size "${width}x${height}" \
        -framerate "$frame_rate" -i "$input" \
        -lavfi "xpsnr=stats_file=$stats" -f null -
    read -r xpsnr_y xpsnr_u xpsnr_v xpsnr_min < <(
      awk '
        /^XPSNR average,/ {
          for (i = 1; i <= NF; i++) {
            if ($i == "y:") y = $(i + 1)
            if ($i == "u:") u = $(i + 1)
            if ($i == "v:") v = $(i + 1)
            if ($i == "(minimum:") {
              minimum = $(i + 1)
              sub(/\)$/, "", minimum)
            }
          }
        }
        END { print y, u, v, minimum }
      ' "$stats"
    )
    if [[ -z "$xpsnr_y" || -z "$xpsnr_min" ]]; then
      echo "failed to parse XPSNR for $id q$quality" >&2
      exit 1
    fi
    printf '%s\t%d\t%d\t%d\t%s\t%s\t%s\t%s\t%s\n' \
      "$id" "$bit_depth" "$quality" "$frames" "$pixel_format" \
      "$xpsnr_y" "$xpsnr_u" "$xpsnr_v" "$xpsnr_min" >> "$output"
  done < <(jq -r '
    .samples[]
    | [
        .id,
        .path,
        (.frames | tostring),
        .frame_rate,
        (.width | tostring),
        (.height | tostring),
        (.pixel_format | capture("yuv422p(?<depth>[0-9]+)le").depth),
        .pixel_format
      ]
    | @tsv
  ' "$manifest")
done

echo "XPSNR results: $output"
