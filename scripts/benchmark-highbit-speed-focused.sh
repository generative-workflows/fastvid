#!/usr/bin/env bash
# Focused balanced high-bit speed-branch comparison; see EXP-0074.
set -euo pipefail

if [[ "$#" -ne 3 ]]; then
  echo "usage: $0 BASELINE_BINARY CANDIDATE_BINARY OUTPUT" >&2
  exit 1
fi

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
baseline="$1"
candidate="$2"
output="$3"
input="$repo_dir/artifacts/corpus-v2/native/high-precision-motion-1280x720-24f-yuv422p10le.raw"
expected_input_hash="ff61ed1af3c39e4b12e8a98a8edb94b2d76e2dfcc2f318a62e111b7080b5fbad"

for binary in "$baseline" "$candidate"; do
  if [[ ! -x "$binary" ]]; then
    echo "binary is not executable: $binary" >&2
    exit 1
  fi
done
if [[ "$(sha256sum "$input" | cut -d' ' -f1)" != "$expected_input_hash" ]]; then
  echo "focused input hash mismatch" >&2
  exit 1
fi
mkdir -p "$(dirname -- "$output")"

first=1
run_variant() {
  local variant="$1"
  local binary="$2"
  local trial="$3"
  local result
  result="$("$binary" benchmark-yuv422p16le \
    "$input" 1280 720 24/1 24 10 "$quality" "$threads" 1)"
  if [[ "$first" == 1 ]]; then
    printf 'variant\ttrial\t%s\n' "$(printf '%s\n' "$result" | head -n 1)" > "$output"
    first=0
  fi
  printf '%s\t%d\t%s\n' \
    "$variant" "$trial" "$(printf '%s\n' "$result" | tail -n 1)" >> "$output"
}

for quality in 90 100; do
  for threads in 1 4; do
    "$baseline" benchmark-yuv422p16le \
      "$input" 1280 720 24/1 24 10 "$quality" "$threads" 1 > /dev/null
    "$candidate" benchmark-yuv422p16le \
      "$input" 1280 720 24/1 24 10 "$quality" "$threads" 1 > /dev/null
    for trial in 1 2 3 4 5 6; do
      if (( trial % 2 == 1 )); then
        run_variant baseline "$baseline" "$trial"
        run_variant candidate "$candidate" "$trial"
      else
        run_variant candidate "$candidate" "$trial"
        run_variant baseline "$baseline" "$trial"
      fi
    done
  done
done

echo "results: $output"
