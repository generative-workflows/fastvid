#!/usr/bin/env bash
# EXP-0071 charged chroma-from-luma fast-feedback model.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
corpus_dir="${1:-$repo_dir/artifacts/corpus-v2}"
output="${2:-$repo_dir/artifacts/exp0071-chroma-screening.tsv}"

cargo build --release --bin chroma_model --manifest-path "$repo_dir/Cargo.toml"
binary="$repo_dir/target/release/chroma_model"
mkdir -p "$(dirname -- "$output")"

temporary_dir="$(mktemp -d /tmp/fastvid-chroma.XXXXXX)"
ui_frame="$temporary_dir/ui-1f.yuv"
cuts_frame="$temporary_dir/cuts-1f.yuv"
cleanup() {
  rm -f -- "$ui_frame" "$cuts_frame"
  rmdir -- "$temporary_dir"
}
trap cleanup EXIT
dd if="$corpus_dir/videos/ui-dashboard-scroll-1280x720-24f.yuv" \
  of="$ui_frame" bs=1843200 count=1 status=none
dd if="$corpus_dir/videos/procedural-scene-cuts-1920x1080-24f.yuv" \
  of="$cuts_frame" bs=4147200 count=1 status=none

cases=(
  $'camera-cholla\tcamera\t'"$corpus_dir"$'/stills/camera-cholla-1920x1080.yuv\t1920\t1080\t24/1'
  $'ai-greenhouse\tai-generated\t'"$corpus_dir"$'/stills/ai-greenhouse-1920x1080.yuv\t1920\t1080\t24/1'
  $'ui-dashboard-scroll\tsynthetic-ui\t'"$ui_frame"$'\t1280\t720\t24/1'
  $'procedural-scene-cuts\tsynthetic-ui\t'"$cuts_frame"$'\t1920\t1080\t24/1'
)

first=1
for case_spec in "${cases[@]}"; do
  IFS=$'\t' read -r sample category input width height frame_rate <<< "$case_spec"
  for quality in 90 100; do
    result="$("$binary" "$sample" "$category" "$input" "$width" "$height" \
      "$frame_rate" 1 "$quality" 1)"
    if [[ "$first" == 1 ]]; then
      printf '%s\n' "$result" | head -n 1 > "$output"
      first=0
    fi
    printf '%s\n' "$result" | tail -n +2 >> "$output"
  done
done

echo "results: $output"
python3 "$script_dir/summarize-chroma-model.py" "$output"
