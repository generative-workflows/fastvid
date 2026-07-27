#!/usr/bin/env bash
# Fully charged shard-local order-0 model over every current-corpus frame.
set -euo pipefail

if [[ "$#" -lt 3 ]]; then
  echo "usage: $0 V5_ENTROPY_MODEL CORPUS OUTPUT.tsv [QUALITY ...]" >&2
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
temporary_dir="$(mktemp -d /tmp/fastvid-entropy-model.XXXXXX)"
trap 'rm -rf -- "$temporary_dir"' EXIT

[[ -x "$binary" ]] || { echo "binary is not executable: $binary" >&2; exit 1; }
mkdir -p "$(dirname -- "$output")"
if [[ ! -s "$output" ]]; then
  printf 'sample\tframe\twidth\theight\tbit_depth\tquality\traw_bytes\tencoded_bytes\tstream_overhead_bytes\tshards\tzero_run_shards\trice_shards\tblock_pack_shards\torder0_supported_shards\torder0_winning_shards\tcurrent_shard_bytes\toracle_shard_bytes\toracle_stream_bytes\toracle_saving_percent\n' > "$output"
fi

while IFS=$'\t' read -r sample relative_path width height frames frame_rate; do
  source="$corpus_dir/$relative_path"
  [[ -f "$source" ]] || { echo "missing corpus sample: $source" >&2; exit 1; }
  input="$temporary_dir/input-yuv422p10le.yuv"
  ffmpeg -nostdin -hide_banner -loglevel error -y \
    -f rawvideo -pixel_format yuv422p -video_size "${width}x${height}" -framerate "$frame_rate" -i "$source" \
    -frames:v "$frames" -pix_fmt yuv422p10le -f rawvideo "$input"
  for quality in "${qualities[@]}"; do
    existing="$(awk -F '\t' -v sample="$sample" -v quality="$quality" '
      NR > 1 && $1 == sample && $6 == quality { count++ }
      END { print count + 0 }
    ' "$output")"
    if [[ "$existing" -eq "$frames" ]]; then
      continue
    fi
    if [[ "$existing" -ne 0 ]]; then
      echo "partial existing cell: $sample q$quality has $existing/$frames frames" >&2
      exit 1
    fi
    row="$temporary_dir/model.tsv"
    "$binary" "$sample" "$input" "$width" "$height" "$frame_rate" 10 "$quality" 1 256 128 "$frames" > "$row"
    awk 'NR > 1' "$row" >> "$output"
  done
done < <(jq -r '
  .samples[] | select(.track == "codec")
  | [.id, .path, (.width | tostring), (.height | tostring), (.frames | tostring), .frame_rate]
  | @tsv
' "$manifest")

echo "v5 entropy model: $output"
