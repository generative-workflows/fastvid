#!/usr/bin/env bash
# Durable CPU baseline for the version-5 CUDA handoff.
set -euo pipefail

if [[ "$#" -lt 3 || "$#" -gt 4 ]]; then
  echo "usage: $0 BINARY CORPUS_DIR OUTPUT_PREFIX [SPEED_TRIALS]" >&2
  exit 2
fi

binary="$1"
corpus_dir="$2"
output_prefix="$3"
speed_trials="${4:-5}"
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
manifest="$repo_dir/corpus/high-bit-manifest.json"
quality_output="${output_prefix}-quality.tsv"
speed_output="${output_prefix}-speed.tsv"
environment_output="${output_prefix}-environment.txt"
temporary="$(mktemp /tmp/fastvid-cpu-gpu-baseline.XXXXXX)"
trap 'rm -f -- "$temporary"' EXIT

if [[ ! -x "$binary" ]]; then
  echo "binary is not executable: $binary" >&2
  exit 1
fi
if (( speed_trials < 1 )); then
  echo "speed trial count must be positive" >&2
  exit 1
fi
(
  cd "$corpus_dir"
  sha256sum --check "$repo_dir/corpus/high-bit-checksums.sha256"
) >/dev/null
mkdir -p "$(dirname -- "$output_prefix")"

run_case() {
  local input="$1"
  local width="$2"
  local height="$3"
  local frame_rate="$4"
  local frames="$5"
  local bit_depth="$6"
  local quality="$7"
  local threads="$8"
  "$binary" benchmark-yuv422p16le-parallel-full-tile \
    "$input" "$width" "$height" "$frame_rate" "$frames" \
    "$bit_depth" "$quality" "$threads" 1
}

first=1
for quality in 60 75 90 95 100; do
  while IFS=$'\t' read -r id path frames frame_rate width height bit_depth; do
    run_case "$corpus_dir/$path" "$width" "$height" "$frame_rate" \
      "$frames" "$bit_depth" "$quality" 1 > "$temporary"
    if (( first == 1 )); then
      {
        printf 'sample\t'
        head -n 1 "$temporary"
      } > "$quality_output"
      first=0
    fi
    printf '%s\t' "$id" >> "$quality_output"
    tail -n 1 "$temporary" >> "$quality_output"
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

first=1
for quality in 90 100; do
  for threads in 1 2 4; do
    while IFS=$'\t' read -r id path frames frame_rate width height bit_depth; do
      input="$corpus_dir/$path"
      run_case "$input" "$width" "$height" "$frame_rate" \
        "$frames" "$bit_depth" "$quality" "$threads" >/dev/null
      for trial in $(seq 1 "$speed_trials"); do
        run_case "$input" "$width" "$height" "$frame_rate" \
          "$frames" "$bit_depth" "$quality" "$threads" > "$temporary"
        if (( first == 1 )); then
          {
            printf 'sample\ttrial\t'
            head -n 1 "$temporary"
          } > "$speed_output"
          first=0
        fi
        printf '%s\t%d\t' "$id" "$trial" >> "$speed_output"
        tail -n 1 "$temporary" >> "$speed_output"
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
done

echo "quality results: $quality_output"
echo "speed results: $speed_output"
{
  echo "utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "git_commit=$(git -C "$repo_dir" rev-parse HEAD)"
  echo "binary=$binary"
  echo "binary_sha256=$(sha256sum "$binary" | cut -d' ' -f1)"
  echo "rustc=$(rustc --version)"
  echo "ffmpeg=$(ffmpeg -version | head -n 1)"
  echo "uname=$(uname -srmo)"
  lscpu
  free -h
} > "$environment_output"
echo "environment: $environment_output"
