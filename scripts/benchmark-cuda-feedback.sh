#!/usr/bin/env bash
# Fast, reproducible CPU-encode/CUDA-decode feedback on two real-world 4K frames.
set -euo pipefail

if [[ "$#" -lt 3 || "$#" -gt 5 ]]; then
  echo "usage: $0 FASTVID CORPUS_V3 OUTPUT_PREFIX [TRIALS [quick|full]]" >&2
  exit 2
fi

binary="$1"
corpus_dir="$2"
output_prefix="$3"
trials="${4:-5}"
scope="${5:-quick}"
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
temporary_dir="$(mktemp -d /tmp/fastvid-cuda-feedback.XXXXXX)"
trap 'rm -rf -- "$temporary_dir"' EXIT

if [[ ! -x "$binary" ]]; then
  echo "binary is not executable: $binary" >&2
  exit 1
fi
if (( trials < 1 )); then
  echo "trial count must be positive" >&2
  exit 1
fi
if [[ "$scope" != quick && "$scope" != full ]]; then
  echo "scope must be quick or full" >&2
  exit 1
fi
for command in ffmpeg nvidia-smi python sha256sum; do
  command -v "$command" >/dev/null || { echo "missing command: $command" >&2; exit 1; }
done
python -c 'import torch, fastvid_cuda; assert torch.cuda.is_available()'

mkdir -p "$(dirname -- "$output_prefix")"
speed_output="${output_prefix}-encode.tsv"
quality_output="${output_prefix}-quality.tsv"
decode_output="${output_prefix}-decode.tsv"
cuda_encode_output="${output_prefix}-cuda-encode.tsv"
environment_output="${output_prefix}-environment.txt"

printf 'sample\ttrial\tinput\tframes\tbit_depth\tquality\tthreads\tgop\ttile_width\ttile_height\traw_bytes\tencoded_bytes\tratio\tencode_ms\tdecode_ms\tencode_mpps\tdecode_mpps\tencode_raw_mb_s\tdecode_raw_mb_s\tencoded_stream_mb_s\tencoded_stream_mbps\ty_psnr\tcb_psnr\tcr_psnr\ty_block_ssim\tmax_error\n' > "$speed_output"
printf 'sample\tquality\traw_bytes\tencoded_bytes\tratio\ty_psnr_db\tcb_psnr_db\tcr_psnr_db\ty_block_ssim\tmax_error\txpsnr_y_db\txpsnr_u_db\txpsnr_v_db\txpsnr_min_db\tcompression_gt_15x\txpsnr_gt_50db\texact\n' > "$quality_output"
printf 'sample\tquality\tinput\tplacement\tpredictor\twidth\theight\tencoded_bytes\traw_bytes\tratio\tmedian_ms\tdecode_gpps\traw_gb_s\n' > "$decode_output"
printf 'sample\tinput\tquality\twidth\theight\tbit_depth\tencoded_bytes\traw_bytes\tratio\tmedian_ms\tencode_gpps\traw_gb_s\n' > "$cuda_encode_output"

field() {
  local key="$1"
  local file="$2"
  awk -F '\t' -v key="$key" '
    NR == 1 { for (i = 1; i <= NF; i++) if ($i == key) column = i; next }
    NR == 2 { if (!column) exit 2; print $column }
  ' "$file"
}

run_benchmark() {
  local input="$1"
  local width="$2"
  local height="$3"
  local frame_rate="$4"
  local quality="$5"
  local threads="$6"
  "$binary" benchmark-yuv422p16le-parallel-full-tile \
    "$input" "$width" "$height" "$frame_rate" 1 10 "$quality" "$threads" 1 256 128
}

manifest="$repo_dir/corpus/manifest.json"
if [[ "$scope" == quick ]]; then
  selection='["bbb-grass-fur", "camera-pontegana", "camera-cholla", "procedural-chroma-edges", "spring-native-2k", "glass-half-native-4k", "people-vote-march-native-4k", "calotes-versicolor-native-4k"]'
else
  selection='null'
fi

while IFS=$'\t' read -r sample relative_path width height frame_rate source_id; do
  source="$corpus_dir/$relative_path"
  if [[ ! -f "$source" ]]; then
    echo "missing corpus sample: $source" >&2
    exit 1
  fi
  input="$temporary_dir/${sample}-yuv422p10le.yuv"
  ffmpeg -nostdin -hide_banner -loglevel error -y \
    -f rawvideo -pixel_format yuv422p -video_size "${width}x${height}" -framerate "$frame_rate" -i "$source" \
    -frames:v 1 -pix_fmt yuv422p10le -f rawvideo "$input"

  for quality in 90 100; do
    quality_row="$temporary_dir/${sample}-q${quality}-quality.tsv"
    run_benchmark "$input" "$width" "$height" "$frame_rate" "$quality" 1 > "$quality_row"

    for threads in 1 4; do
      run_benchmark "$input" "$width" "$height" "$frame_rate" "$quality" "$threads" >/dev/null
      for trial in $(seq 1 "$trials"); do
        trial_row="$temporary_dir/${sample}-q${quality}-t${threads}-${trial}.tsv"
        run_benchmark "$input" "$width" "$height" "$frame_rate" "$quality" "$threads" > "$trial_row"
        printf '%s\t%d\t' "$sample" "$trial" >> "$speed_output"
        tail -n 1 "$trial_row" >> "$speed_output"
      done
    done

    stream="$temporary_dir/${sample}-q${quality}.fvid"
    decoded="$temporary_dir/${sample}-q${quality}-decoded.yuv"
    "$binary" encode-yuv422p16le-parallel-full-tile \
      "$input" "$stream" "$width" "$height" "$frame_rate" 10 "$quality" 1 256 128
    "$binary" decode16 "$stream" "$decoded" 1

    cuda_encode_row="$temporary_dir/${sample}-q${quality}-cuda-encode.tsv"
    python "$repo_dir/cuda/benchmarks/benchmark_encode_v5.py" \
      "$input" "$width" "$height" 10 "$quality" \
      --frame-rate "$frame_rate" --warmups 3 --trials "$trials" --oracle "$stream" \
      > "$cuda_encode_row"
    printf '%s\t' "$sample" >> "$cuda_encode_output"
    tail -n 1 "$cuda_encode_row" >> "$cuda_encode_output"

    stats="$temporary_dir/${sample}-q${quality}-xpsnr.log"
    ffmpeg -nostdin -hide_banner -loglevel error \
      -f rawvideo -pixel_format yuv422p10le -video_size "${width}x${height}" -framerate "$frame_rate" -i "$decoded" \
      -f rawvideo -pixel_format yuv422p10le -video_size "${width}x${height}" -framerate "$frame_rate" -i "$input" \
      -lavfi "xpsnr=stats_file=$stats" -f null -
    read -r xpsnr_y xpsnr_u xpsnr_v xpsnr_min < <(
      awk '
        /^XPSNR average,/ {
          for (i = 1; i <= NF; i++) {
            if ($i == "y:") y = $(i + 1)
            if ($i == "u:") u = $(i + 1)
            if ($i == "v:") v = $(i + 1)
            if ($i == "(minimum:") { minimum = $(i + 1); sub(/\)$/, "", minimum) }
          }
        }
        END { print y, u, v, minimum }
      ' "$stats"
    )
    [[ -n "$xpsnr_y" && -n "$xpsnr_min" ]] || { echo "failed to parse XPSNR" >&2; exit 1; }

    raw_bytes="$(field raw_bytes "$quality_row")"
    encoded_bytes="$(field encoded_bytes "$quality_row")"
    ratio="$(field ratio "$quality_row")"
    y_psnr="$(field y_psnr "$quality_row")"
    cb_psnr="$(field cb_psnr "$quality_row")"
    cr_psnr="$(field cr_psnr "$quality_row")"
    ssim="$(field y_block_ssim "$quality_row")"
    max_error="$(field max_error "$quality_row")"
    compression_pass="$(awk -v value="$ratio" 'BEGIN { print (value > 15.0 ? "true" : "false") }')"
    xpsnr_pass="$(awk -v value="$xpsnr_y" 'BEGIN { print (value > 50.0 || value == "inf" ? "true" : "false") }')"
    exact="$(awk -v value="$max_error" 'BEGIN { print (value == 0 ? "true" : "false") }')"
    printf '%s\t%d\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
      "$sample" "$quality" "$raw_bytes" "$encoded_bytes" "$ratio" \
      "$y_psnr" "$cb_psnr" "$cr_psnr" "$ssim" "$max_error" \
      "$xpsnr_y" "$xpsnr_u" "$xpsnr_v" "$xpsnr_min" \
      "$compression_pass" "$xpsnr_pass" "$exact" >> "$quality_output"

    decode_row="$temporary_dir/${sample}-q${quality}-decode.tsv"
    python "$repo_dir/cuda/benchmarks/benchmark_decode_v5.py" \
      "$stream" --warmups 5 --trials "$trials" > "$decode_row"
    tail -n +2 "$decode_row" | while IFS= read -r row; do
      printf '%s\t%d\t%s\n' "$sample" "$quality" "$row" >> "$decode_output"
    done
  done
done < <(jq -r --argjson selection "$selection" '
  .samples[]
  | select(.track == "codec")
  | select($selection == null or (.id as $id | $selection | index($id)))
  | [.id, .path, (.width | tostring), (.height | tostring), .frame_rate, .source]
  | @tsv
' "$manifest")

{
  echo "utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "git_commit=$(git -C "$repo_dir" rev-parse HEAD)"
  echo "git_dirty=$(git -C "$repo_dir" status --porcelain | wc -l)"
  echo "scope=$scope"
  echo "trials=$trials"
  echo "corpus_manifest_sha256=$(sha256sum "$manifest" | cut -d' ' -f1)"
  echo "binary=$binary"
  echo "binary_sha256=$(sha256sum "$binary" | cut -d' ' -f1)"
  extension="$(python -c 'import torch; import fastvid_cuda._C as module; print(module.__file__)')"
  echo "cuda_extension=$extension"
  echo "cuda_extension_sha256=$(sha256sum "$extension" | cut -d' ' -f1)"
  echo "rustc=$(rustc --version)"
  echo "python=$(python --version 2>&1)"
  echo "pytorch=$(python -c 'import torch; print(torch.__version__)')"
  echo "cuda_runtime=$(python -c 'import torch; print(torch.version.cuda)')"
  echo "ffmpeg=$(ffmpeg -version | head -n 1)"
  nvidia-smi --query-gpu=name,compute_cap,driver_version,memory.total,clocks.sm,power.limit --format=csv,noheader
  lscpu
  free -h
} > "$environment_output"

echo "encode trials: $speed_output"
echo "quality gates: $quality_output"
echo "CUDA decode: $decode_output"
echo "CUDA encode: $cuda_encode_output"
echo "environment: $environment_output"
