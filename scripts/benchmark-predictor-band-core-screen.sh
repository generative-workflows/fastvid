#!/usr/bin/env bash
# Representative 8-bit corpus screen for the predictor-band ladder.
set -euo pipefail

if [[ "$#" -lt 1 || "$#" -gt 3 ]]; then
  echo "usage: $0 OUTPUT [CORPUS] [QUALITY]" >&2
  exit 1
fi

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
output="$1"
corpus_dir="${2:-$repo_dir/artifacts/corpus-v2}"
quality="${3:-90}"
binary="$repo_dir/target/release/predictor_model"
manifest="$repo_dir/corpus/manifest.json"
temporary="$(mktemp /tmp/fastvid-predictor-band-core.XXXXXX)"
trap 'rm -f -- "$temporary"' EXIT

mkdir -p "$(dirname -- "$output")"
cargo build --release --bin predictor_model > /dev/null
first=1
while IFS=$'\t' read -r id path frames frame_rate width height; do
  "$binary" yuv422 "$id" "$corpus_dir/$path" "$width" "$height" \
    "$frame_rate" "$frames" "$quality" 1 1 > "$temporary"
  if (( first == 1 )); then
    cp "$temporary" "$output"
    first=0
  else
    tail -n +2 "$temporary" >> "$output"
  fi
done < <(jq -r '
  .samples[]
  | select(.id == "camera-cholla"
        or .id == "ai-greenhouse"
        or .id == "ui-dashboard-scroll"
        or .id == "noisy-camera-fourpeople"
        or .id == "resolution-grid-4k")
  | [
      .id,
      .path,
      (.frames | tostring),
      .frame_rate,
      (.width | tostring),
      (.height | tostring)
    ]
  | @tsv
' "$manifest")

echo "results: $output"
