# EXP-0150 — corpus-v4 UVG-VCM/Xiph 4K expansion

Status: **ACCEPTED**

Date: 2026-07-27

## Hypothesis

Adding short raw/native-master 4K windows with traffic, people, indoor sports,
foliage, and distinct motion levels will expose rate-quality behavior hidden
by corpus-v3's rendered and previously compressed real-world 4K sources,
without making routine full-corpus feedback depend on multi-gigabyte complete
sequences.

## Modification

Create corpus-v4 as a strict superset of v3. Add four checksummed, frame-aligned
24-frame 3840x2160 excerpts:

- UVG-VCM Highway View frames 120–143;
- UVG-VCM Floorball Train frames 300–323;
- Xiph/SVT ParkJoy frames 0–23;
- Xiph/SVT IntoTree frames 0–23.

Fetch only exact HTTP byte ranges, retain the source license notices, convert
deterministically to canonical limited-range BT.709 YUV422p8, and record both
source-range and derived hashes.

## Test

- Verify every ranged source byte count and SHA-256.
- Decode exactly 24 complete frames at the declared dimensions/rates.
- Review representative frames and signal statistics for truncation,
  endianness, range, and conversion errors.
- Verify all v3 derived hashes remain unchanged.
- Run first-frame and full-frame rate-quality feedback on all 28 samples /
  350 frames, with separate 4K reporting.

## Gate

Accept corpus-v4 if generation is reproducible, all source/derived checksums
pass, old samples remain byte-identical, and the four additions produce valid
nonduplicate frame sequences. Do not promote a source whose license or color
conversion cannot be recorded explicitly.

## Results

Corpus-v4 contains 28 codec samples from 14 sources and 350 frames: 170 4K,
130 1080p, 25 native-2K, 24 720p, and one 360p frame. All entries in
`corpus/derived-checksums.sha256`, including every inherited v3 entry, pass.

Each addition is exactly 398,131,200 bytes and contains 24/24 unique frames:

| Sample | Derived SHA-256 |
|---|---|
| UVG-VCM Highway View | `0d97cbffeda28e6837651e48bb123ab6b1c47fbf76d5352f97502263bd02ef17` |
| UVG-VCM Floorball Train | `33b0fc41bb77530914615e7e23ba39472f6bf87871ff9728f09d9b06cee43d49` |
| Xiph/SVT ParkJoy | `da8af64935d0d4dbb13b21ef993971329b03d271b703638292c46cc7ee516dee` |
| Xiph/SVT IntoTree | `7ebe7401cc5e844f96f6bbc1e545791a5f82b27115f53a05d4fdf4ffbae54914` |

Source-range hashes, exact HTTP ranges, color assumptions, source notices,
and conversion commands are retained in the manifest and fetch script.
EXP-0151 completed all 1,750 rate/quality rows over the expanded corpus.

## Decision

Accepted. Corpus-v4 becomes the current evaluation corpus. The Xiph/SVT
material remains evaluation-only under its accompanying test-material notice;
it is not a code dependency or redistributable MIT project asset.

## References

- [Research 0045](../research/0045-open-4k-video-subsections.md)
- [Evaluation methodology](../EVALUATION_METHODOLOGY.md)
