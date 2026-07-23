# EXP-0008 — Corpus v2 diversity expansion

Status: **ACCEPTED**

## Hypothesis

Adding lossless camera photographs and deterministic screen, chroma, grain,
and scene-cut samples will expose at least one compression or throughput
outlier not visible in corpus v1 while keeping the standard set small enough
for routine development runs.

This follows [research 0009](../research/0009-corpus-v2-diversity.md) and the
pending corpus harness work in [EXP-0003](EXP-0003-regression-corpus.md).

## Modification

- Preserve corpus v1's manifest and derivative checksums.
- Define corpus v2 as a strict superset with two CC0 lossless TIFF camera
  stills.
- Add deterministic canonical YUV422p8 generation for chroma edges, scrolling
  screen-style content, temporally changing grain, and hard scene cuts.
- Add a prompt-pinned AI-generated still, a public-domain camera clip with
  controlled sensor noise, and a 360p/720p/1080p/4K resolution ladder.
- Retain source-native 10-bit BT.2020/PQ HDR and RGBA alpha diagnostics as
  explicitly unsupported capability rows rather than flattening them.
- Pin external sources and all generated derivatives with SHA-256.
- Update fetch, validation, documentation, and benchmark defaults to v2.

## Test

Regenerate corpus v2 from an empty destination and verify every checksum.
Confirm every sample is exactly 1920x1080 planar limited-range YUV422p8 with
the declared frame count. Run quality-90 and quality-100 one-thread smoke
benchmarks and report per-sample outliers relative to corpus-v1 ranges.

## Results

Host: 4-vCPU AMD EPYC-Genoa VM, 7.6 GiB RAM, Rust 1.97.1, FFmpeg
8.0.1. Corpus v2 regenerated successfully into 845 MiB of source cache,
canonical derivatives, and native diagnostics. All twenty derivative/native
files match their committed SHA-256 values and their manifest-derived byte
counts.

The codec track now has twelve stills and six 24-frame videos. The resolution
set is 640x360, 1280x720, 1920x1080, and 3840x2160. The separate capability
track contains one 4K YUV444p10 BT.2020/PQ frame and one 1024x1024 RGBA frame;
both are correctly excluded from YUV422p8 headline scores.

Single-trial one-thread development summaries:

| Quality | Samples | Geo. ratio | Mean encode | Mean decode | Mean Y PSNR | Mean SSIM |
|---:|---:|---:|---:|---:|---:|---:|
| 90 | 18 | 7.308x | 27.90 MP/s | 55.41 MP/s | 49.868 dB | 0.996557 |
| 100 | 18 | 5.013x | 27.38 MP/s | 54.01 MP/s | exact | 1.000000 |

The expansion exposed strong new outliers. At quality 100, `camera-pontegana`
compresses only 1.777x, `camera-cholla` 1.842x, noisy `FourPeople` 1.970x, and
the AI greenhouse 2.001x; corpus v1's worst sample was 2.765x. Conversely, the
UI clip reaches 55.238x and the chroma-edge still 288.421x, demonstrating that
a single corpus-wide geometric mean hides major content-class differences.
The camera stills are also the slowest encodes at roughly 19–21 MP/s.

All fourteen Rust tests, strict Clippy, manifest parsing, shell syntax,
derivative byte-count checks, and SHA-256 verification pass.

## Decision

Accept. Corpus v2 fills the requested camera, noisy footage, synthetic
graphics/UI, AI-generated, animation, grain, scene-cut, HDR, alpha, and
resolution gaps while preserving corpus v1 unchanged. The outlier hypothesis
passes clearly. Future benchmark summaries must group natural, procedural/AI,
and resolution classes rather than treating the improved aggregate ratio as a
codec improvement.
