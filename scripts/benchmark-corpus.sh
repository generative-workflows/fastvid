#!/usr/bin/env bash
# Standard matrix: EVALUATION_METHODOLOGY.md and research/0006.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
corpus_dir="${1:-$repo_dir/artifacts/corpus-v1}"
output="${2:-$repo_dir/artifacts/corpus-results.tsv}"
qualities="${3:-60 75 90 95 100}"
thread_counts="${4:-1 4}"
gop="${5:-1}"
kind="${6:-all}"
trials="${7:-5}"

cargo build --release --manifest-path "$repo_dir/Cargo.toml"
binary="$repo_dir/target/release/fastvid"
manifest="$repo_dir/corpus/manifest.json"
mkdir -p "$(dirname -- "$output")"

first=1
for quality in $qualities; do
  for threads in $thread_counts; do
    while IFS=$'\t' read -r path frames frame_rate; do
      "$binary" benchmark-yuv422 \
        "$corpus_dir/$path" 1920 1080 "$frame_rate" "$frames" "$quality" "$threads" "$gop" \
        > /dev/null
      for trial in $(seq 1 "$trials"); do
        result="$("$binary" benchmark-yuv422 \
          "$corpus_dir/$path" 1920 1080 "$frame_rate" "$frames" "$quality" "$threads" "$gop")"
        if [[ "$first" == 1 ]]; then
          printf 'trial\t%s\n' "$(printf '%s\n' "$result" | head -n 1)" > "$output"
          first=0
        fi
        printf '%d\t%s\n' "$trial" "$(printf '%s\n' "$result" | tail -n 1)" >> "$output"
      done
    done < <(jq -r --arg kind "$kind" '
      . as $manifest
      | .samples[]
      | select($kind == "all" or .kind == $kind)
      | .source as $source
      | [$manifest.sources[] | select(.id == $source) | .frame_rate] as $rate
      | [.path, (.frames | tostring), $rate[0]]
      | @tsv
    ' "$manifest")
  done
done

echo "results: $output"
