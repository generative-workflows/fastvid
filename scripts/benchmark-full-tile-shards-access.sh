#!/usr/bin/env bash
# EXP-0110 warm-cache independent tile access comparison.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
output="${1:-$repo_dir/artifacts/exp0110-full-tile-shards-access.tsv}"
corpus_dir="${2:-$repo_dir/artifacts/corpus-v2}"
iterations="${3:-20}"
input="$corpus_dir/native/high-precision-motion-1280x720-24f-yuv422p10le.raw"
binary="$repo_dir/target/release/fastvid"
temporary="$(mktemp /tmp/fastvid-full-tile-shards-access.XXXXXX)"
trap 'rm -f -- "$temporary"' EXIT

mkdir -p "$(dirname -- "$output")"
cargo build --release --bin fastvid --manifest-path "$repo_dir/Cargo.toml" > /dev/null
first=1
for variant in baseline bounded-full-tile; do
  "$binary" benchmark-tile-access-yuv422p16le "$input" 1280 720 24/1 \
    10 90 "$iterations" "$variant" > "$temporary"
  if (( first == 1 )); then
    cp "$temporary" "$output"
    first=0
  else
    tail -n 1 "$temporary" >> "$output"
  fi
done

echo "results: $output"
python3 "$script_dir/summarize-full-tile-shards-access.py" "$output"
