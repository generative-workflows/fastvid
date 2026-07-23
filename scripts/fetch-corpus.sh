#!/usr/bin/env bash
# Corpus provenance and conversion follow research/0006-standard-evaluation-methodology.md.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_dir="$(cd -- "$script_dir/.." && pwd)"
destination="${1:-$repo_dir/artifacts/corpus-v1}"
sources="$destination/sources"
licenses="$destination/licenses"
mkdir -p "$sources/bbb" "$sources/ed" "$licenses" "$destination/stills" "$destination/videos"

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
  curl --fail --location --silent --show-error \
    "$base_url/$filename" --output "$output.part"
  actual="$(sha256sum "$output.part" | awk '{ print $1 }')"
  if [[ "$actual" != "$expected" ]]; then
    echo "checksum mismatch for $filename" >&2
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

convert_still "$sources/bbb/big_buck_bunny_03000.png" "$destination/stills/bbb-grass-fur-03000.yuv"
convert_still "$sources/bbb/big_buck_bunny_09000.png" "$destination/stills/bbb-foliage-sky-09000.yuv"
convert_still "$sources/bbb/big_buck_bunny_12000.png" "$destination/stills/bbb-credits-text-12000.yuv"
convert_still "$sources/ed/03000.png" "$destination/stills/ed-monochrome-lines-03000.yuv"
convert_still "$sources/ed/06000.png" "$destination/stills/ed-dark-motion-06000.yuv"
convert_still "$sources/ed/12000.png" "$destination/stills/ed-dense-warm-12000.yuv"

convert_clip "$sources/bbb/big_buck_bunny_%05d.png" 2989 "$destination/videos/bbb-grass-motion-02989-24f.yuv"
convert_clip "$sources/bbb/big_buck_bunny_%05d.png" 8989 "$destination/videos/bbb-foliage-motion-08989-24f.yuv"
convert_clip "$sources/ed/%05d.png" 11989 "$destination/videos/ed-dense-motion-11989-24f.yuv"

if [[ -f "$repo_dir/corpus/derived-checksums.sha256" ]]; then
  (cd "$destination" && sha256sum --check "$repo_dir/corpus/derived-checksums.sha256")
fi

echo "corpus ready: $destination"
