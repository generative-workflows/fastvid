#!/usr/bin/env bash
# Serial EXP-0065 source-reference block-motion potential screen.
set -euo pipefail

if [[ "$#" -lt 2 || "$#" -gt 5 ]]; then
  echo "usage: $0 BINARY OUTPUT [CORPUS] [QUALITY] [GOP]" >&2
  exit 1
fi

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
binary="$1"
output="$2"
corpus_dir="${3:-$repo_dir/artifacts/corpus-v2}"
quality="${4:-90}"
gop="${5:-12}"
manifest="$repo_dir/corpus/manifest.json"
mkdir -p "$(dirname -- "$output")"

first=1
while IFS=$'\t' read -r sample path frames frame_rate width height; do
  result="$("$binary" \
    "$sample" "$corpus_dir/$path" "$width" "$height" "$frame_rate" \
    "$frames" "$quality" "$gop")"
  if [[ "$first" == 1 ]]; then
    printf '%s\n' "$result" > "$output"
    first=0
  else
    printf '%s\n' "$result" | tail -n +2 >> "$output"
  fi
done < <(jq -r '
  .samples[]
  | select(.track == "codec" and .benchmark != false and .kind == "video")
  | [
      (.path | split("/")[-1] | sub("\\.yuv$"; "")),
      .path,
      (.frames | tostring),
      .frame_rate,
      (.width | tostring),
      (.height | tostring)
    ]
  | @tsv
' "$manifest")

echo "results: $output"
