#!/usr/bin/env bash
# Balanced q90 comparison of two already-built Fastvid binaries.
set -euo pipefail

if (( $# < 8 || $# > 9 )); then
  echo "usage: $0 OUTPUT CORPUS REFERENCE_BINARY CANDIDATE_BINARY REFERENCE_LABEL CANDIDATE_LABEL TRIALS MIN_ENCODE_RATIO [MANIFEST]" >&2
  exit 2
fi

output="$1"
corpus_dir="$2"
reference_binary="$3"
candidate_binary="$4"
reference_label="$5"
candidate_label="$6"
trials="$7"
minimum_encode_ratio="$8"
manifest="${9:-corpus/high-bit-manifest.json}"
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
temporary="$(mktemp /tmp/fastvid-high-bit-binary-ab.XXXXXX)"
trap 'rm -f -- "$temporary"' EXIT

mkdir -p "$(dirname -- "$output")"
first=1
for trial in $(seq 1 "$trials"); do
  while IFS=$'\t' read -r id path frames frame_rate width height bit_depth; do
    if (( trial % 2 == 1 )); then
      variants=("$reference_label" "$candidate_label")
    else
      variants=("$candidate_label" "$reference_label")
    fi
    for variant in "${variants[@]}"; do
      if [[ "$variant" == "$reference_label" ]]; then
        binary="$reference_binary"
      else
        binary="$candidate_binary"
      fi
      "$binary" benchmark-yuv422p16le-parallel-full-tile \
        "$corpus_dir/$path" "$width" "$height" "$frame_rate" "$frames" \
        "$bit_depth" 90 1 1 > "$temporary"
      if (( first == 1 )); then
        {
          printf 'variant\ttrial\tsample\t'
          head -n 1 "$temporary"
        } > "$output"
        first=0
      fi
      printf '%s\t%s\t%s\t' "$variant" "$trial" "$id" >> "$output"
      tail -n 1 "$temporary" >> "$output"
    done
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
done

echo "results: $output"
python3 "$script_dir/summarize-winner-only-shards.py" \
  "$output" "$output" "$minimum_encode_ratio" "$reference_label" "$candidate_label"
