#!/usr/bin/env bash
# Balanced warm-cache single-frame access comparison for the 8-bit video corpus.
set -euo pipefail

if [[ "$#" -lt 3 || ("$#" -gt 7 && "$#" -ne 11) ]]; then
  echo "usage: $0 BASELINE_BINARY CANDIDATE_BINARY OUTPUT [CORPUS] [QUALITIES] [GOPS] [TRIALS [BASELINE_TILE_WIDTH BASELINE_TILE_HEIGHT CANDIDATE_TILE_WIDTH CANDIDATE_TILE_HEIGHT]]" >&2
  exit 1
fi

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
baseline_binary="$1"
candidate_binary="$2"
output="$3"
corpus_dir="${4:-$repo_dir/artifacts/corpus-v2}"
qualities="${5:-90 100}"
gops="${6:-1 12}"
trials="${7:-6}"
baseline_geometry=()
candidate_geometry=()
if [[ "$#" -eq 11 ]]; then
  baseline_geometry=("${8}" "${9}")
  candidate_geometry=("${10}" "${11}")
fi
manifest="$repo_dir/corpus/manifest.json"
targets="0,1,6,11,12,13,18,23"
mkdir -p "$(dirname -- "$output")"

if (( trials < 2 || trials % 2 != 0 )); then
  echo "TRIALS must be a positive even number for balanced execution order" >&2
  exit 1
fi

printf '%s\n' \
  $'variant\ttrial\tinput\ttarget_frame\tkeyframe_frame\tdependency_frames\tdecoded_frames\tquality\tthreads\tgop\tencoded_bytes_read\taccess_ms\tuseful_mpps\twork_mpps\tuseful_raw_mb_s\taccess_amplification' \
  > "$output"

run_variant() {
  local variant="$1"
  local binary="$2"
  local trial="$3"
  local geometry=()
  local result
  if [[ "$variant" == "baseline" ]]; then
    geometry=("${baseline_geometry[@]}")
  else
    geometry=("${candidate_geometry[@]}")
  fi
  result="$("$binary" benchmark-access-yuv422 \
    "$corpus_dir/$path" "$width" "$height" "$frame_rate" "$frames" \
    "$quality" 1 "$gop" "$targets" "${geometry[@]}")"
  printf '%s\n' "$result" | awk -F $'\t' -v variant="$variant" -v trial="$trial" '
    NR == 1 {
      for (field = 1; field <= NF; field++) {
        column[$field] = field
      }
      next
    }
    {
      printf "%s\t%d", variant, trial
      split("input target_frame keyframe_frame dependency_frames decoded_frames quality threads gop encoded_bytes_read access_ms useful_mpps work_mpps useful_raw_mb_s access_amplification", names, " ")
      for (field = 1; field <= length(names); field++) {
        printf "\t%s", $(column[names[field]])
      }
      printf "\n"
    }
  ' >> "$output"
}

while IFS=$'\t' read -r path frames frame_rate width height; do
  for quality in $qualities; do
    for gop in $gops; do
      for trial in $(seq 1 "$trials"); do
        if (( trial % 2 == 1 )); then
          run_variant baseline "$baseline_binary" "$trial"
          run_variant candidate "$candidate_binary" "$trial"
        else
          run_variant candidate "$candidate_binary" "$trial"
          run_variant baseline "$baseline_binary" "$trial"
        fi
      done
    done
  done
done < <(jq -r '
  .samples[]
  | select(.track == "codec" and .benchmark != false and .kind == "video")
  | [.path, (.frames | tostring), .frame_rate, (.width | tostring), (.height | tostring)]
  | @tsv
' "$manifest")

echo "results: $output"
