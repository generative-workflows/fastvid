# EXP-0006 — Tile-local temporal prediction gating

Status: **REJECTED**

## Hypothesis

Selecting spatial or co-located previous-frame prediction independently for
each tile will improve geometric-mean compression on the three corpus-v1
videos by at least 3% at qualities 90 and 100 relative to EXP-0005's
frame-level gate. Reconstruction metrics must remain identical, and
one-thread encode/decode throughput may not regress by more than 5%.

This follows [research 0008](../research/0008-block-local-inter-intra-selection.md)
and the accepted [EXP-0005](EXP-0005-gated-temporal-prediction.md).

## Modification

- Always make the preceding reconstructed frame available on predicted
  frames.
- For each plane tile, use co-located temporal prediction when that tile's
  mean absolute sample difference is at most five levels; otherwise retain
  Paeth spatial prediction.
- Reuse the existing per-tile prediction-mode directory byte, reference-aware
  decoder, GOP structure, and entropy modes; do not change bitstream syntax.
- Specialize the temporal encoder path so it does not allocate or write a
  spatial reconstruction buffer that temporal prediction cannot read.
- Add mixed-mode and high-motion/static-region tests.

## Test

Run the standard corpus-v1 video subset at qualities 90 and 100, GOP 12, with
one and four threads. Compare against the immutable EXP-0005 TSV artifacts.
Preserve per-sample encoded size, PSNR, SSIM, maximum error, and throughput.

Development command shape:

```sh
scripts/benchmark-corpus.sh artifacts/corpus-v1 RESULT.tsv QUALITY THREADS 12 video 1
```

Accept only if both quality levels pass the compression and one-thread speed
gates. Otherwise reject and retain the measurements as evidence for a
different mode-cost estimator or smaller prediction blocks.

## Results

Host: 4-vCPU AMD EPYC-Genoa VM, 7.6 GiB RAM, Rust 1.97.1. Single-trial
development measurements used the same corpus, GOP, and command shape as
EXP-0005. Quality metrics and maximum errors were identical to the baseline.

| Quality | Mode | Geo. ratio | Encode 1T | Decode 1T | Bitrate |
|---:|---|---:|---:|---:|---:|
| 90 | frame gate | 6.550x | 40.25 MP/s | 72.43 MP/s | 127.56 Mb/s |
| 90 | tile gate | 6.294x | 43.09 MP/s | 74.24 MP/s | 136.32 Mb/s |
| 100 | frame gate | 4.008x | 35.13 MP/s | 61.50 MP/s | 206.73 Mb/s |
| 100 | tile gate | 3.836x | 36.89 MP/s | 62.15 MP/s | 221.75 Mb/s |

The grass clip shrank by 3.1% at quality 90 and 2.3% at quality 100, and the
foliage clip was unchanged. The dense-motion clip expanded by 16.3% and 16.9%
respectively. Corpus geometric compression therefore regressed 3.9% at
quality 90 and 4.3% at quality 100, failing the required 3% improvement even
though encode throughput improved 7.0% and 5.0%.

All fifteen Rust tests, strict Clippy, release builds, and the three existing
Lean proofs passed.

## Decision

Reject. Mean absolute difference at the 256x128 tile scale does not predict
residual coding cost reliably enough. A future local selector needs a direct
entropy-cost estimate, smaller decision blocks, or motion compensation. The
temporal-path allocation reduction is separable and will be measured on its
own rather than retained through this rejected experiment.
