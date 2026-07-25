#!/usr/bin/env bash
# Charged reversible-squeeze screening; see EXP-0075.
set -euo pipefail

if [[ "$#" -ne 2 ]]; then
  echo "usage: $0 SQUEEZE_MODEL_BINARY OUTPUT" >&2
  exit 1
fi

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
binary="$1"
output="$2"
manifest="$repo_dir/corpus/high-bit-manifest.json"
corpus_dir="$repo_dir/artifacts/corpus-v2"

if [[ ! -x "$binary" ]]; then
  echo "squeeze model binary is not executable: $binary" >&2
  exit 1
fi
mkdir -p "$(dirname -- "$output")"

first=1
while IFS=$'\t' read -r id path frames frame_rate width height bit_depth; do
  result="$("$binary" yuv422p16le "$id" "$corpus_dir/$path" \
    "$width" "$height" "$frame_rate" "$frames" "$bit_depth")"
  if [[ "$first" == 1 ]]; then
    printf '%s\n' "$result" > "$output"
    first=0
  else
    printf '%s\n' "$result" | tail -n +2 >> "$output"
  fi
done < <(jq -r '
  .samples[]
  | [
      .id,
      .path,
      (.frames | tostring),
      .frame_rate,
      (.width | tostring),
      (.height | tostring),
      (.pixel_format | capture("yuv422p(?<depth>[0-9]+)le").depth)
    ]
  | @tsv
' "$manifest")

echo "results: $output"
