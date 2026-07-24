#!/usr/bin/env bash
# EXP-0053 fast-feedback order-0 residual model.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
corpus_dir="${1:-$repo_dir/artifacts/corpus-v2}"
output="${2:-$repo_dir/artifacts/exp0053-order0-screening.tsv}"

cargo build --release --bin entropy_model --manifest-path "$repo_dir/Cargo.toml"
binary="$repo_dir/target/release/entropy_model"
mkdir -p "$(dirname -- "$output")"

temporary_dir="$(mktemp -d /tmp/fastvid-order0.XXXXXX)"
ui_prefix="$temporary_dir/ui-4f.yuv"
cuts_prefix="$temporary_dir/cuts-4f.yuv"
cleanup() {
  rm -f -- "$ui_prefix" "$cuts_prefix"
  rmdir -- "$temporary_dir"
}
trap cleanup EXIT
dd if="$corpus_dir/videos/ui-dashboard-scroll-1280x720-24f.yuv" \
  of="$ui_prefix" bs=7372800 count=1 status=none
dd if="$corpus_dir/videos/procedural-scene-cuts-1920x1080-24f.yuv" \
  of="$cuts_prefix" bs=16588800 count=1 status=none

cases=(
  $'camera-1080p\t'"$corpus_dir"$'/stills/camera-cholla-1920x1080.yuv\t1920\t1080\t24/1\t1\t1'
  $'grid-4k\t'"$corpus_dir"$'/stills/resolution-grid-3840x2160.yuv\t3840\t2160\t24/1\t1\t1'
  $'ui-temporal-720p\t'"$ui_prefix"$'\t1280\t720\t24/1\t4\t12'
  $'cuts-temporal-1080p\t'"$cuts_prefix"$'\t1920\t1080\t24/1\t4\t12'
)

first=1
for case_spec in "${cases[@]}"; do
  IFS=$'\t' read -r sample input width height frame_rate frames gop <<< "$case_spec"
  for quality in 90 100; do
    result="$("$binary" yuv422 "$sample" "$input" "$width" "$height" \
      "$frame_rate" "$frames" "$quality" 1 "$gop")"
    if [[ "$first" == 1 ]]; then
      printf '%s\n' "$result" | head -n 1 > "$output"
      first=0
    fi
    printf '%s\n' "$result" | tail -n +2 >> "$output"
  done
done

echo "results: $output"
python3 "$script_dir/summarize-order0-model.py" "$output"

