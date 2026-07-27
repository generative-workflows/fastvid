#!/usr/bin/env bash
# Corpus provenance and conversion follow research/0006-standard-evaluation-methodology.md.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
destination="${1:-$repo_dir/artifacts/corpus-v4}"
sources="$destination/sources"
licenses="$destination/licenses"
review_cache="$repo_dir/artifacts/corpus-source-review"
mkdir -p "$sources/bbb" "$sources/ed" "$sources/external" "$licenses" \
  "$destination/stills" "$destination/videos" "$destination/native"

curl --fail --location --silent --show-error \
  https://media.xiph.org/BBB/BBB-1080-png/SHA256SUMS.txt \
  --output "$licenses/bbb-SHA256SUMS.txt"
curl --fail --location --silent --show-error \
  https://media.xiph.org/BBB/BBB-1080-png/README.txt \
  --output "$licenses/bbb-README.txt"
curl --fail --location --silent --show-error \
  https://media.xiph.org/ED/ED-1080-png/SHA256SUMS \
  --output "$licenses/ed-SHA256SUMS.txt"
curl --fail --location --silent --show-error \
  https://media.xiph.org/ED/ED-1080-png/README.txt \
  --output "$licenses/ed-README.txt"

download_frame() {
  local source_id="$1"
  local filename="$2"
  local base_url manifest output expected actual
  if [[ "$source_id" == "bbb" ]]; then
    base_url="https://media.xiph.org/BBB/BBB-1080-png"
    manifest="$licenses/bbb-SHA256SUMS.txt"
  else
    base_url="https://media.xiph.org/ED/ED-1080-png"
    manifest="$licenses/ed-SHA256SUMS.txt"
  fi
  output="$sources/$source_id/$filename"
  expected="$(awk -v name="$filename" '$2 == name { print $1 }' "$manifest")"
  if [[ -z "$expected" ]]; then
    echo "missing upstream checksum for $filename" >&2
    return 1
  fi
  if [[ -f "$output" ]]; then
    actual="$(sha256sum "$output" | awk '{ print $1 }')"
    if [[ "$actual" == "$expected" ]]; then
      return
    fi
  fi
  if [[ -f "$repo_dir/artifacts/corpus-v1/sources/$source_id/$filename" ]]; then
    cp "$repo_dir/artifacts/corpus-v1/sources/$source_id/$filename" "$output"
    actual="$(sha256sum "$output" | awk '{ print $1 }')"
    if [[ "$actual" == "$expected" ]]; then
      return
    fi
  fi
  curl --fail --location --silent --show-error \
    "$base_url/$filename" --output "$output.part"
  actual="$(sha256sum "$output.part" | awk '{ print $1 }')"
  if [[ "$actual" != "$expected" ]]; then
    echo "checksum mismatch for $filename" >&2
    return 1
  fi
  mv "$output.part" "$output"
}

download_verified() {
  local url="$1"
  local expected="$2"
  local filename="$3"
  local output="$sources/external/$filename"
  local actual
  for candidate in "$output" "$review_cache/$filename"; do
    if [[ -f "$candidate" ]]; then
      actual="$(sha256sum "$candidate" | awk '{ print $1 }')"
      if [[ "$actual" == "$expected" ]]; then
        if [[ "$candidate" != "$output" ]]; then
          cp "$candidate" "$output"
        fi
        return
      fi
    fi
  done
  curl --fail --location --silent --show-error "$url" --output "$output.part"
  actual="$(sha256sum "$output.part" | awk '{ print $1 }')"
  if [[ "$actual" != "$expected" ]]; then
    echo "checksum mismatch for $filename" >&2
    return 1
  fi
  mv "$output.part" "$output"
}

download_range_verified() {
  local url="$1"
  local byte_range="$2"
  local expected="$3"
  local filename="$4"
  local output="$sources/external/$filename"
  local actual
  for candidate in "$output" "$review_cache/$filename"; do
    if [[ -f "$candidate" ]]; then
      actual="$(sha256sum "$candidate" | awk '{ print $1 }')"
      if [[ "$actual" == "$expected" ]]; then
        if [[ "$candidate" != "$output" ]]; then
          cp "$candidate" "$output"
        fi
        return
      fi
    fi
  done
  curl --fail --location --silent --show-error --range "$byte_range" \
    "$url" --output "$output.part"
  actual="$(sha256sum "$output.part" | awk '{ print $1 }')"
  if [[ "$actual" != "$expected" ]]; then
    echo "checksum mismatch for ranged source $filename" >&2
    return 1
  fi
  mv "$output.part" "$output"
}

for number in 03000 09000 12000; do
  download_frame bbb "big_buck_bunny_${number}.png"
done
for number in 03000 06000 12000; do
  download_frame ed "${number}.png"
done
for value in $(seq 2989 3012) $(seq 8989 9012); do
  number="$(printf '%05d' "$value")"
  download_frame bbb "big_buck_bunny_${number}.png"
done
for value in $(seq 11989 12012); do
  number="$(printf '%05d' "$value")"
  download_frame ed "${number}.png"
done

download_verified \
  "https://commons.wikimedia.org/wiki/Special:Redirect/file/Pontegana2.tif" \
  "af17026fbf3979b58370ce2a877dd4ecd71f454121a1b00735238e41b0be03c7" \
  "Pontegana2.tif"
download_verified \
  "https://commons.wikimedia.org/wiki/Special:Redirect/file/Cane_Cholla_Las_Cruces_NM.tiff" \
  "4d63c000d1785c5752b9cb7c2a533686fd0c8e2e6e78e075d4b3c881be3fffcb" \
  "Cane_Cholla_Las_Cruces_NM.tiff"
download_verified \
  "https://media.xiph.org/video/derf/webm/FourPeople_1280x720_60.webm" \
  "1e4a5df7e67ae985370321cc2c91b8595f7e3433ad8ee1e84da7a26ff5254deb" \
  "FourPeople_1280x720_60.webm"
download_verified \
  "https://upload.wikimedia.org/wikipedia/commons/a/a5/Spring_-_Blender_Open_Movie.webm" \
  "d691a199035cc7d295210b286f8f6734893c7d4358d228081af6f0da98a56343" \
  "spring-2048x858.webm"
download_verified \
  "https://upload.wikimedia.org/wikipedia/commons/0/02/Glass_Half_-_Blender_Open_Movie-full_movie.webm" \
  "d11b4cc23a973e758ff7c45cce6fef0c287eab4ba248f9080cbd22a07014626b" \
  "glass-half-3840x2160.webm"
download_verified \
  "https://upload.wikimedia.org/wikipedia/commons/2/27/2019-03-23_People%27s_Vote_March_-_Put_It_to_the_People.webm" \
  "3bb0ea3f13d856d04ddabfc6f50a40ef43d7fd61a6091126c16939fe6aa8eed8" \
  "people-vote-march-3840x2160.webm"
download_verified \
  "https://upload.wikimedia.org/wikipedia/commons/3/38/Calotes_versicolor.webm" \
  "7712d15746b415be04feb58471ebe59bf29766b76a275958ed4468d0c4813cf5" \
  "calotes-versicolor-3840x2160.webm"
download_range_verified \
  "https://ultravideo.fi/UVG-VCM/HighwayView/HighwayView_3840x2160_60fps_yuv444_16bits_600.yuv" \
  "5971968000-7166361599" \
  "076415adb4e8c4599b19c97af5b69ed58dede17a820dfdf07dbb16765ec8da1a" \
  "uvg-vcm-highway-view-f120-143.yuv"
download_range_verified \
  "https://ultravideo.fi/UVG-VCM/FloorballTrain/FloorballTrain_3840x2160_60fps_yuv444_16bits_600.yuv" \
  "14929920000-16124313599" \
  "8a657cfdfc2eb6a790d627a3fb2bf37512bf2ad007c2884005a253fce3e50447" \
  "uvg-vcm-floorball-train-f300-323.yuv"
download_range_verified \
  "https://media.xiph.org/video/derf/y4m/park_joy_2160p50.y4m" \
  "0-298598579" \
  "7a2fc73b86e9d9e28d511dc9e0fc47674aadcb53c3ec0529974499adf8ddd2b9" \
  "xiph-park-joy-f000-023-partial.y4m"
download_range_verified \
  "https://media.xiph.org/video/derf/y4m/in_to_tree_2160p50.y4m" \
  "0-298598579" \
  "1676283d9f060dab0d99cc8df0867c1f4d60d9a77541b81eff9a98c7758edad5" \
  "xiph-into-tree-f000-023-partial.y4m"
download_verified \
  "https://media.xiph.org/video/derf/vqeg.its.bldrdoc.gov/HDTV/SVT_MultiFormat/SVT_MultiFormat_v10.pdf" \
  "9fd39b9db02375a086f8e65129177d4db37f275712cc3ace56fb71d007f9f13f" \
  "SVT_MultiFormat_v10.pdf"
download_verified \
  "https://creativecommons.org/licenses/by/4.0/legalcode.txt" \
  "9ba9550ad48438d0836ddab3da480b3b69ffa0aac7b7878b5a0039e7ab429411" \
  "CC-BY-4.0-legalcode.txt"
cp "$sources/external/SVT_MultiFormat_v10.pdf" "$licenses/SVT_MultiFormat_v10.pdf"
cp "$sources/external/CC-BY-4.0-legalcode.txt" "$licenses/CC-BY-4.0-legalcode.txt"

convert_still() {
  local input="$1"
  local output="$2"
  ffmpeg -v error -y -sws_flags lanczos+accurate_rnd+full_chroma_int \
    -i "$input" \
    -vf "scale=in_range=pc:out_range=tv:out_color_matrix=bt709" \
    -pix_fmt yuv422p -frames:v 1 -f rawvideo "$output"
}

convert_clip() {
  local input_pattern="$1"
  local start="$2"
  local output="$3"
  ffmpeg -v error -y -sws_flags lanczos+accurate_rnd+full_chroma_int \
    -framerate 24 -start_number "$start" -i "$input_pattern" \
    -vf "scale=in_range=pc:out_range=tv:out_color_matrix=bt709" \
    -pix_fmt yuv422p -frames:v 24 -f rawvideo "$output"
}

convert_external_still() {
  local input="$1"
  local output="$2"
  ffmpeg -v error -y -sws_flags lanczos+accurate_rnd+full_chroma_int \
    -i "$input" \
    -vf "scale=1920:1080:force_original_aspect_ratio=increase:in_range=pc:out_range=tv:out_color_matrix=bt709,crop=1920:1080" \
    -pix_fmt yuv422p -frames:v 1 -f rawvideo "$output"
}

convert_native_video() {
  local input="$1"
  local timestamp="$2"
  local frames="$3"
  local output="$4"
  ffmpeg -v error -y -sws_flags lanczos+accurate_rnd+full_chroma_int \
    -ss "$timestamp" -i "$input" \
    -vf "fps=24,scale=iw:ih:in_range=tv:out_range=tv:in_color_matrix=bt709:out_color_matrix=bt709,format=yuv422p" \
    -frames:v "$frames" -f rawvideo "$output"
}

convert_uvg_vcm_video() {
  local input="$1"
  local output="$2"
  ffmpeg -v error -y -sws_flags lanczos+accurate_rnd+full_chroma_int \
    -f rawvideo -pixel_format yuv444p16le -video_size 3840x2160 \
    -framerate 60 -i "$input" \
    -vf "scale=in_range=pc:out_range=tv:in_color_matrix=bt709:out_color_matrix=bt709,format=yuv422p" \
    -frames:v 24 -f rawvideo "$output"
}

convert_xiph_svt_video() {
  local input="$1"
  local output="$2"
  ffmpeg -v error -y -sws_flags lanczos+accurate_rnd+full_chroma_int \
    -i "$input" \
    -vf "scale=in_range=tv:out_range=tv:in_color_matrix=bt709:out_color_matrix=bt709,format=yuv422p" \
    -frames:v 24 -f rawvideo "$output"
}

convert_still "$sources/bbb/big_buck_bunny_03000.png" "$destination/stills/bbb-grass-fur-03000.yuv"
convert_still "$sources/bbb/big_buck_bunny_09000.png" "$destination/stills/bbb-foliage-sky-09000.yuv"
convert_still "$sources/bbb/big_buck_bunny_12000.png" "$destination/stills/bbb-credits-text-12000.yuv"
convert_still "$sources/ed/03000.png" "$destination/stills/ed-monochrome-lines-03000.yuv"
convert_still "$sources/ed/06000.png" "$destination/stills/ed-dark-motion-06000.yuv"
convert_still "$sources/ed/12000.png" "$destination/stills/ed-dense-warm-12000.yuv"

convert_clip "$sources/bbb/big_buck_bunny_%05d.png" 2989 "$destination/videos/bbb-grass-motion-02989-24f.yuv"
convert_clip "$sources/bbb/big_buck_bunny_%05d.png" 8989 "$destination/videos/bbb-foliage-motion-08989-24f.yuv"
convert_clip "$sources/ed/%05d.png" 11989 "$destination/videos/ed-dense-motion-11989-24f.yuv"

convert_external_still \
  "$sources/external/Pontegana2.tif" \
  "$destination/stills/camera-pontegana-1920x1080.yuv"
convert_external_still \
  "$sources/external/Cane_Cholla_Las_Cruces_NM.tiff" \
  "$destination/stills/camera-cholla-1920x1080.yuv"
convert_external_still \
  "$repo_dir/corpus/sources/ai-greenhouse.png" \
  "$destination/stills/ai-greenhouse-1920x1080.yuv"

ffmpeg -v error -y -sws_flags lanczos+accurate_rnd+full_chroma_int \
  -ss 2 -i "$sources/external/FourPeople_1280x720_60.webm" \
  -vf "fps=24,scale=1920:1080:in_range=tv:out_range=tv:out_color_matrix=bt709,format=yuv422p,noise=alls=8:allf=t+u:all_seed=424242" \
  -pix_fmt yuv422p -frames:v 24 -f rawvideo \
  "$destination/videos/noisy-camera-fourpeople-1920x1080-24f.yuv"

# Native-size v3 controls. The two real-world sources omit color primaries in
# their WebM stream metadata; the deterministic conversion therefore records
# and applies the corpus BT.709 limited-range assumption explicitly.
convert_native_video "$sources/external/spring-2048x858.webm" 180 1 \
  "$destination/stills/spring-2048x858.yuv"
convert_native_video "$sources/external/glass-half-3840x2160.webm" 60 1 \
  "$destination/stills/glass-half-3840x2160.yuv"
convert_native_video "$sources/external/spring-2048x858.webm" 240 24 \
  "$destination/videos/spring-2048x858-24f.yuv"
convert_native_video "$sources/external/glass-half-3840x2160.webm" 120 24 \
  "$destination/videos/glass-half-3840x2160-24f.yuv"
convert_native_video "$sources/external/people-vote-march-3840x2160.webm" 8 24 \
  "$destination/videos/people-vote-march-3840x2160-24f.yuv"
convert_native_video "$sources/external/calotes-versicolor-3840x2160.webm" 5 24 \
  "$destination/videos/calotes-versicolor-3840x2160-24f.yuv"
convert_uvg_vcm_video \
  "$sources/external/uvg-vcm-highway-view-f120-143.yuv" \
  "$destination/videos/uvg-vcm-highway-view-3840x2160-f120-143.yuv"
convert_uvg_vcm_video \
  "$sources/external/uvg-vcm-floorball-train-f300-323.yuv" \
  "$destination/videos/uvg-vcm-floorball-train-3840x2160-f300-323.yuv"
convert_xiph_svt_video \
  "$sources/external/xiph-park-joy-f000-023-partial.y4m" \
  "$destination/videos/xiph-park-joy-3840x2160-f000-023.yuv"
convert_xiph_svt_video \
  "$sources/external/xiph-into-tree-f000-023-partial.y4m" \
  "$destination/videos/xiph-into-tree-3840x2160-f000-023.yuv"

cargo build --release --manifest-path "$repo_dir/Cargo.toml" --bin corpusgen
"$repo_dir/target/release/corpusgen" "$destination"

if [[ -f "$repo_dir/corpus/derived-checksums.sha256" ]]; then
  (cd "$destination" && sha256sum --check "$repo_dir/corpus/derived-checksums.sha256")
fi
if [[ -f "$repo_dir/corpus/high-bit-checksums.sha256" ]]; then
  (cd "$destination" && sha256sum --check "$repo_dir/corpus/high-bit-checksums.sha256")
fi

echo "corpus ready: $destination"
