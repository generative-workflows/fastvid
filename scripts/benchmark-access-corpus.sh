#!/usr/bin/env bash
# Warm-cache codec-only single-frame access: research/0010 and EXP-0009.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
corpus_dir="${1:-$repo_dir/artifacts/corpus-v2}"
output="${2:-$repo_dir/artifacts/access-results.tsv}"
qualities="${3:-90 100}"
thread_counts="${4:-1 4}"
gops="${5:-1 12}"
trials="${6:-5}"

cargo build --release --manifest-path "$repo_dir/Cargo.toml" --bin fastvid
binary="$repo_dir/target/release/fastvid"
manifest="$repo_dir/corpus/manifest.json"
mkdir -p "$(dirname -- "$output")"

first=1
for quality in $qualities; do
  for threads in $thread_counts; do
    for gop in $gops; do
      while IFS=$'\t' read -r path frames frame_rate width height; do
        targets="0,1,6,11,12,13,18,23"
        result="$("$binary" benchmark-access-yuv422 \
          "$corpus_dir/$path" "$width" "$height" "$frame_rate" "$frames" \
          "$quality" "$threads" "$gop" "$targets")"
        for trial in $(seq 1 "$trials"); do
          result="$("$binary" benchmark-access-yuv422 \
            "$corpus_dir/$path" "$width" "$height" "$frame_rate" "$frames" \
            "$quality" "$threads" "$gop" "$targets")"
          if [[ "$first" == 1 ]]; then
            printf 'trial\t%s\n' "$(printf '%s\n' "$result" | head -n 1)" > "$output"
            first=0
          fi
          printf '%s\n' "$result" | tail -n +2 | while IFS= read -r row; do
            printf '%d\t%s\n' "$trial" "$row" >> "$output"
          done
        done
      done < <(jq -r '
        .samples[]
        | select(.track == "codec" and .benchmark != false and .kind == "video")
        | [.path, (.frames | tostring), .frame_rate, (.width | tostring), (.height | tostring)]
        | @tsv
      ' "$manifest")
    done
  done
done

if [[ "$first" == 1 ]]; then
  echo "no access benchmark samples selected" >&2
  exit 1
fi
echo "results: $output"
