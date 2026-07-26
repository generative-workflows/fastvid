#!/usr/bin/env bash
# EXP-0106 one-frame spatial residual-order model.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
corpus_dir="${1:-$repo_dir/artifacts/corpus-v2}"
output="${2:-$repo_dir/artifacts/exp0106-diagonal-order.tsv}"
binary="$repo_dir/target/release/entropy_model"
manifest="$repo_dir/corpus/manifest.json"
high_bit_manifest="$repo_dir/corpus/high-bit-manifest.json"
temporary="$(mktemp /tmp/fastvid-diagonal-order.XXXXXX)"
trap 'rm -f -- "$temporary"' EXIT

mkdir -p "$(dirname -- "$output")"
cargo build --release --bin entropy_model --manifest-path "$repo_dir/Cargo.toml" \
  > /dev/null
first=1

append_first_frame() {
  local mode="$1"
  local id="$2"
  local input="$3"
  local width="$4"
  local height="$5"
  local frame_rate="$6"
  local bit_depth="$7"
  local bytes_per_sample="$8"
  local frame_samples=$((width * height + 2 * ((width + 1) / 2) * height))
  local frame_bytes=$((frame_samples * bytes_per_sample))

  head -c "$frame_bytes" "$input" > "$temporary"
  if [[ "$mode" == yuv422 ]]; then
    result="$("$binary" yuv422 "$id" "$temporary" "$width" "$height" \
      "$frame_rate" 1 90 1 1)"
  else
    result="$("$binary" yuv422p16le "$id" "$temporary" "$width" "$height" \
      "$frame_rate" 1 "$bit_depth" 90 1 1)"
  fi
  if (( first == 1 )); then
    printf '%s\n' "$result" > "$output"
    first=0
  else
    printf '%s\n' "$result" | tail -n +2 >> "$output"
  fi
}

while IFS=$'\t' read -r id path width height frame_rate; do
  append_first_frame yuv422 "$id" "$corpus_dir/$path" "$width" "$height" \
    "$frame_rate" 8 1
done < <(jq -r '
  .samples[]
  | select(.id == "camera-cholla"
      or .id == "ai-greenhouse"
      or .id == "noisy-camera-fourpeople"
      or .id == "ui-dashboard-scroll"
      or .id == "resolution-grid-4k")
  | [.id, .path, .width, .height, .frame_rate]
  | @tsv
' "$manifest")

while IFS=$'\t' read -r id path width height frame_rate pixel_format; do
  bit_depth="${pixel_format#yuv422p}"
  bit_depth="${bit_depth%le}"
  append_first_frame yuv422p16le "$id" "$corpus_dir/$path" "$width" "$height" \
    "$frame_rate" "$bit_depth" 2
done < <(jq -r '
  .samples[]
  | [.id, .path, .width, .height, .frame_rate, .pixel_format]
  | @tsv
' "$high_bit_manifest")

echo "results: $output"
python3 "$script_dir/summarize-diagonal-order-model.py" "$output"
