# Open 4K video subsections for corpus-v4

## Sources reviewed

- [UVG-VCM](https://ultravideo.fi/UVG-VCM/index.html), Ultra Video Group,
  Tampere University. The dataset page identifies 20 uncompressed annotated
  sequences, mostly 3840x2160 at 60 fps in 16-bit YUV444, under CC-BY-4.0.
- [Classic UVG Dataset](https://ultravideo.fi/dataset.html). It is technically
  useful but explicitly CC-BY-NC, so it is excluded from Fastvid's corpus.
- [Xiph Derf test-media collection](https://media.xiph.org/video/derf/).
- [SVT Multi Format Test Set notice](https://media.xiph.org/video/derf/vqeg.its.bldrdoc.gov/HDTV/SVT_MultiFormat/SVT_MultiFormat_v10.pdf).
  It documents 3840x2160p50 BT.709 masters and permits use for developing,
  testing, and presenting technology standards, subject to notice and
  no-fee-distribution conditions.

## Selection

Corpus-v4 adds four 24-frame 3840x2160 windows:

| Source | Frames | Motion/content | Ranged-source SHA-256 |
|---|---:|---|---|
| UVG-VCM Highway View | 120–143 | traffic, people, foliage, outdoor camera motion | `076415adb4e8c4599b19c97af5b69ed58dede17a820dfdf07dbb16765ec8da1a` |
| UVG-VCM Floorball Train | 300–323 | people, fast indoor sports motion, textured floor | `8a657cfdfc2eb6a790d627a3fb2bf37512bf2ad007c2884005a253fce3e50447` |
| Xiph/SVT ParkJoy | 0–23 | difficult high-detail/high-motion film content | `7a2fc73b86e9d9e28d511dc9e0fc47674aadcb53c3ec0529974499adf8ddd2b9` |
| Xiph/SVT IntoTree | 0–23 | lower-motion foliage and camera movement | `1676283d9f060dab0d99cc8df0867c1f4d60d9a77541b81eff9a98c7758edad5` |

HTTP byte ranges avoid fetching 6–53 GB complete sources. UVG-VCM ranges are
raw frame-aligned YUV444p16le. Xiph ranges contain the exact YUV4MPEG header
and 24 complete 2160p50 YUV420 frame records.

## Color and conversion guardrails

The UVG-VCM page does not signal primaries, transfer, matrix, or range for the
raw files. Corpus conversion therefore records an explicit full-range BT.709
assumption and converts to limited-range YUV422p8. The Xiph/SVT notice
documents BT.709; its YUV4MPEG files are treated as limited-range BT.709 and
chroma-upsampled to the canonical YUV422p8 layout.

These assumptions are corpus metadata, not evidence that the current v5
bitstream carries color semantics. The future format metadata requirement
remains separate.

## Relevant experiments

- [EXP-0150: corpus-v4 UVG-VCM/Xiph expansion](../experiments/EXP-0150-corpus-v4-uvg-xiph.md)
