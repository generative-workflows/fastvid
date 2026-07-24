#!/usr/bin/env bash
# EXP-0056 complete 8-bit q90/q100 causal-context residual model.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
corpus_dir="${1:-$repo_dir/artifacts/corpus-v2}"
output="${2:-$repo_dir/artifacts/exp0056-context-order0-model.tsv}"
manifest="$repo_dir/corpus/manifest.json"

cargo build --release --bin entropy_model --manifest-path "$repo_dir/Cargo.toml"
binary="$repo_dir/target/release/entropy_model"
mkdir -p "$(dirname -- "$output")"
first=1

while IFS=$'\t' read -r id kind path width height frames frame_rate; do
  if [[ "$kind" == video ]]; then
    gop=12
  else
    gop=1
  fi
  for quality in 90 100; do
    result="$("$binary" yuv422 "$id" "$corpus_dir/$path" "$width" "$height" \
      "$frame_rate" "$frames" "$quality" 1 "$gop")"
    if [[ "$first" == 1 ]]; then
      printf '%s\n' "$result" | head -n 1 > "$output"
      first=0
    fi
    printf '%s\n' "$result" | tail -n +2 >> "$output"
  done
done < <(
  jq -r '.samples[] |
    select(.track == "codec" and .benchmark != false) |
    [.id, .kind, .path, .width, .height, .frames, .frame_rate] | @tsv' "$manifest"
)

echo "results: $output"
python3 "$script_dir/summarize-context-order0-model.py" "$output"
