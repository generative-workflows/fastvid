#!/usr/bin/env bash
# Balanced isolated high-bit decode comparison of two complete Fastvid binaries.
set -euo pipefail

if (( $# != 10 )); then
  echo "usage: $0 OUTPUT INPUT REFERENCE_BINARY CANDIDATE_BINARY REFERENCE_LABEL CANDIDATE_LABEL TRIALS REPETITIONS THREADS MIN_DECODE_RATIO" >&2
  exit 2
fi

output="$1"
input="$2"
reference_binary="$3"
candidate_binary="$4"
reference_label="$5"
candidate_label="$6"
trials="$7"
repetitions="$8"
threads="$9"
minimum_decode_ratio="${10}"
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
temporary="$(mktemp /tmp/fastvid-decode-binary-ab.XXXXXX)"
trap 'rm -f -- "$temporary"' EXIT

mkdir -p "$(dirname -- "$output")"
"$reference_binary" benchmark-decode16 "$input" "$threads" 1 >/dev/null
"$candidate_binary" benchmark-decode16 "$input" "$threads" 1 >/dev/null

first=1
for trial in $(seq 1 "$trials"); do
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
    "$binary" benchmark-decode16 "$input" "$threads" "$repetitions" >"$temporary"
    if (( first == 1 )); then
      {
        printf 'variant\ttrial\t'
        head -n 1 "$temporary"
      } >"$output"
      first=0
    fi
    printf '%s\t%s\t' "$variant" "$trial" >>"$output"
    tail -n 1 "$temporary" >>"$output"
  done
done

echo "results: $output"
python3 "$script_dir/summarize-decode-binary-ab.py" \
  "$output" "$reference_label" "$candidate_label" "$minimum_decode_ratio"
