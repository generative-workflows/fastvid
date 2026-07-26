#!/usr/bin/env bash
# EXP-0111 candidate-only q90 feedback against fixed EXP-0110 rows.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
output="${1:-$repo_dir/artifacts/exp0111-winner-only-shards.tsv}"
corpus_dir="${2:-$repo_dir/artifacts/corpus-v2}"
trials="${3:-2}"
reference="${4:-$repo_dir/artifacts/exp0110-full-tile-shards-ab.tsv}"
manifest="$repo_dir/corpus/high-bit-manifest.json"
binary="$repo_dir/target/release/fastvid"
temporary="$(mktemp /tmp/fastvid-winner-only-shards.XXXXXX)"
trap 'rm -f -- "$temporary"' EXIT

mkdir -p "$(dirname -- "$output")"
cargo build --release --bin fastvid --manifest-path "$repo_dir/Cargo.toml" > /dev/null
first=1
for trial in $(seq 1 "$trials"); do
  while IFS=$'\t' read -r id path frames frame_rate width height bit_depth; do
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
    printf 'winner-only\t%s\t%s\t' "$trial" "$id" >> "$output"
    tail -n 1 "$temporary" >> "$output"
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
python3 "$script_dir/summarize-winner-only-shards.py" "$reference" "$output"
