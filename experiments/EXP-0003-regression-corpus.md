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

Pending corpus licensing and harness implementation.

