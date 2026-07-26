#!/usr/bin/env bash
# EXP-0108 native high-bit version-2/version-4 comparison.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
output="${1:-$repo_dir/artifacts/exp0108-bounded-shard-ab.tsv}"
corpus_dir="${2:-$repo_dir/artifacts/corpus-v2}"
trials="${3:-1}"
manifest="$repo_dir/corpus/high-bit-manifest.json"
binary="$repo_dir/target/release/fastvid"
temporary="$(mktemp /tmp/fastvid-bounded-shard-ab.XXXXXX)"
trap 'rm -f -- "$temporary"' EXIT

mkdir -p "$(dirname -- "$output")"
cargo build --release --bin fastvid --manifest-path "$repo_dir/Cargo.toml" > /dev/null
first=1
for trial in $(seq 1 "$trials"); do
  while IFS=$'\t' read -r id path frames frame_rate width height bit_depth; do
    for quality in 90 100; do
      for variant in baseline bounded-shard; do
        command=benchmark-yuv422p16le
        if [[ "$variant" == bounded-shard ]]; then
          command=benchmark-yuv422p16le-parallel
        fi
        "$binary" "$command" "$corpus_dir/$path" "$width" "$height" \
          "$frame_rate" "$frames" "$bit_depth" "$quality" 1 1 > "$temporary"
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
python3 "$script_dir/summarize-bounded-shard-ab.py" "$output"
