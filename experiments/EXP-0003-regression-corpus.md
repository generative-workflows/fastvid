# EXP-0003 — Reproducible regression corpus and rate-distortion harness

Status: **PENDING**

## Hypothesis

A small provenance-checked corpus spanning natural detail, motion, gradients,
noise, screen content, and high-frequency edges will expose codec changes that
the current smooth synthetic fixture hides, and a multi-quality harness will
prevent accepting improvements that merely overfit one content class.

This follows [EXP-0002](EXP-0002-zero-run-tokens.md) and the
[evaluation-methodology review](../research/0004-codec-evaluation.md).

## Modification

- Select short lossless/publicly redistributable 4:2:2 or convertible source
  clips only after checking clip-specific terms.
- Record URL, license, SHA-256, pixel format, dimensions, frame rate, and frame
  range in a machine-readable manifest.
- Add luma/chroma PSNR, luma MS-SSIM, encoded bytes, wall time, and throughput
  output at multiple quality settings.
- Keep large media outside Git; provide a fetch/verification script only for
  clearly licensed clips.

## Test

Run at least four content classes at qualities 60, 75, 90, 95, and 100 with
one and four threads. Compare single-frame/tile random access and full-frame
decode. Treat any >5% lossless expansion on a corpus clip as a prompt to test
an adaptive entropy mode.

## Results

In progress. Corpus v1 now provides six 1920x1080 stills and three 24-frame
1920x1080 clips from SHA-256-verified, CC-BY lossless Blender Open Movie PNG
masters. Canonical YUV422p8 derivatives, exact hashes, provenance, fetch and
conversion tooling, a machine-readable manifest, and a sequence benchmark
command are implemented. Benchmark rows now record both luma MP/s and actual
raw-byte encode/decode throughput, plus encoded-stream MB/s and Mb/s derived
from source duration. For canonical even-width YUV422p8, 1 MP/s corresponds
to 2 decimal MB/s.

The complete five-quality, one/four-thread development matrix exposed the
synthetic-fixture bias. Selected one-thread corpus summaries:

| Quality | Geo. ratio | Mean encode | Mean decode | Mean Y PSNR | Mean block SSIM |
|---:|---:|---:|---:|---:|---:|
| 60 | 9.435x | 28.73 MP/s | 66.85 MP/s | 40.364 dB | 0.965007 |
| 75 | 5.712x | 25.72 MP/s | 58.94 MP/s | 42.946 dB | 0.973432 |
| 90 | 5.797x | 26.82 MP/s | 61.58 MP/s | 49.787 dB | 0.994824 |
| 95 | 3.440x | 24.98 MP/s | 56.26 MP/s | 51.420 dB | 0.994981 |
| 100 | 3.641x | 24.41 MP/s | 57.22 MP/s | exact | 1.000000 |

No quality-100 sample expands; the worst is `ed-dense-motion` at 2.765x.
Quality 90 misses the methodology's 10x high-fidelity target, establishing a
clear optimization objective. The harness now performs a warm-up and five
recorded trials by default.

The experiment remains pending until luma MS-SSIM is implemented, corpus-v2
coverage from EXP-0008 is fully benchmarked, and the standard five-trial
results are recorded. EXP-0009 now covers the previously missing single-frame
access portion of this experiment's test plan.
