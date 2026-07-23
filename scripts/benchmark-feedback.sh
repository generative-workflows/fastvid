#!/usr/bin/env bash
# Sub-minute optimization feedback loop; see EXP-0010.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
corpus_dir="${1:-$repo_dir/artifacts/corpus-v2}"
output="${2:-$repo_dir/artifacts/feedback-results.tsv}"
trials="${3:-5}"

if [[ "${FASTVID_SKIP_BUILD:-0}" != 1 ]]; then
  cargo build --release --manifest-path "$repo_dir/Cargo.toml"
fi
binary="$repo_dir/target/release/fastvid"
mkdir -p "$(dirname -- "$output")"

temporary_dir="$(mktemp -d /tmp/fastvid-feedback.XXXXXX)"
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
  $'grid-4k\tstills/resolution-grid-3840x2160.yuv\t3840\t2160\t24/1\t1\t100\t1\t1'
  $'camera-1080p\tstills/camera-cholla-1920x1080.yuv\t1920\t1080\t24/1\t1\t90\t1\t1'
  $'ui-temporal-720p\t'"$ui_prefix"$'\t1280\t720\t24/1\t4\t90\t4\t12'
  $'cuts-temporal-1080p\t'"$cuts_prefix"$'\t1920\t1080\t24/1\t4\t90\t1\t12'
)

first=1
for case_spec in "${cases[@]}"; do
  IFS=$'\t' read -r case_id path width height frame_rate frames quality threads gop \
    <<< "$case_spec"
  if [[ "$path" == /* ]]; then
    input="$path"
  else
    input="$corpus_dir/$path"
  fi
  if [[ ! -f "$input" ]]; then
    echo "missing feedback corpus sample: $input" >&2
    exit 1
  fi
  "$binary" benchmark-yuv422 \
    "$input" "$width" "$height" "$frame_rate" "$frames" "$quality" "$threads" "$gop" \
    > /dev/null
  for trial in $(seq 1 "$trials"); do
    result="$("$binary" benchmark-yuv422 \
      "$input" "$width" "$height" "$frame_rate" "$frames" "$quality" "$threads" "$gop")"
    if [[ "$first" == 1 ]]; then
      printf 'case\ttrial\t%s\n' "$(printf '%s\n' "$result" | head -n 1)" > "$output"
      first=0
    fi
    printf '%s\t%d\t%s\n' \
      "$case_id" "$trial" "$(printf '%s\n' "$result" | tail -n 1)" >> "$output"
  done
done

echo "results: $output"
"$script_dir/summarize-feedback.awk" "$output"
