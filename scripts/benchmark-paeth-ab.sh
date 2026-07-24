#!/usr/bin/env bash
# Focused alternating A/B benchmark for EXP-0059.
set -euo pipefail

if [[ "$#" -lt 3 || "$#" -gt 5 ]]; then
  echo "usage: $0 OUTPUT BASELINE CANDIDATE [CORPUS] [TRIALS]" >&2
  exit 1
fi

output="$1"
baseline="$2"
candidate="$3"
corpus="${4:-artifacts/corpus-v2/videos/noisy-camera-fourpeople-1920x1080-24f.yuv}"
trials="${5:-6}"

if [[ ! -x "$baseline" || ! -x "$candidate" ]]; then
  echo "baseline and candidate must be executable" >&2
  exit 1
fi

mkdir -p "$(dirname -- "$output")"
printf 'variant\ttrial\t%s\n' \
  "$("$baseline" benchmark-yuv422 "$corpus" 1920 1080 24/1 24 90 1 1 |
    head -n 1)" > "$output"
"$candidate" benchmark-yuv422 "$corpus" 1920 1080 24/1 24 90 1 1 > /dev/null

run_variant() {
  local variant="$1"
  local trial="$2"
  local binary="$3"
  local result
  result="$("$binary" benchmark-yuv422 "$corpus" 1920 1080 24/1 24 90 1 1)"
  printf '%s\t%d\t%s\n' "$variant" "$trial" \
    "$(printf '%s\n' "$result" | tail -n 1)" >> "$output"
}

for trial in $(seq 1 "$trials"); do
  if (( trial % 2 == 1 )); then
    run_variant baseline "$trial" "$baseline"
    run_variant candidate "$trial" "$candidate"
  else
    run_variant candidate "$trial" "$candidate"
    run_variant baseline "$trial" "$baseline"
  fi
done

echo "results: $output"
