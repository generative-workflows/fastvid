# EXP-0005 — Gated previous-frame temporal prediction

Status: **ACCEPTED**

## Hypothesis

For the corpus-v1 video subset, co-located previous-frame prediction with a
12-frame key interval will improve geometric-mean compression by at least 15%
at qualities 90 and 100 without changing reconstruction quality. A luma-SAD
gate will prevent the large expansion expected on high-motion material while
retaining temporal speed gains.

This follows the standard video track in
[research 0006](../research/0006-standard-evaluation-methodology.md) and the
[temporal DPCM review](../research/0007-temporal-dpcm-gating.md).

## Modification

- Add a per-tile prediction selector in a formerly reserved directory byte.
- Prediction mode one subtracts the co-located sample in the previous
  reconstructed frame; mode zero retains Paeth.
- Add reference-aware encode/decode APIs with dimension/format validation.
- Reset to spatial keyframes every 12 frames in the sequence harness.
- Select temporal coding only when mean absolute luma frame difference is at
  most five levels; otherwise encode the frame spatially.
- Add malformed-mode, missing/mismatched-reference, exact temporal round-trip,
  and high-motion fallback tests.
- Specify and prove temporal residual recomposition in Lean.

## Test

Compare all-intra and GOP-12 coding on all three 24-frame corpus-v1 videos at
qualities 90 and 100 with one and four threads. Preserve identical PSNR, SSIM,
and maximum error. The acceptance gate uses the geometric mean of compression
ratios and arithmetic mean throughput over the video subset.

Development command shape:

```sh
scripts/benchmark-corpus.sh artifacts/corpus-v1 RESULT.tsv QUALITY THREADS 12 video 1
```

## Results

Host: 4-vCPU AMD EPYC-Genoa VM, 7.6 GiB RAM, Rust 1.97.1. These are
single-trial development measurements; the standard five-trial protocol is
retained in the harness for release comparisons.

| Quality | Mode | Geo. ratio | Encode 1T | Decode 1T | Encode 4T | Decode 4T |
|---:|---|---:|---:|---:|---:|---:|
| 90 | all-intra | 5.190x | 25.94 MP/s | 59.81 MP/s | 96.58 MP/s | 199.17 MP/s |
| 90 | gated GOP-12 | 6.550x | 40.25 MP/s | 72.43 MP/s | 135.79 MP/s | 230.12 MP/s |
| 100 | all-intra | 3.242x | 24.45 MP/s | 56.39 MP/s | 90.39 MP/s | 186.72 MP/s |
| 100 | gated GOP-12 | 4.008x | 35.13 MP/s | 61.50 MP/s | 116.72 MP/s | 199.26 MP/s |

At quality 90, compression improves 26.2%, one-thread encode throughput 55.2%,
and one-thread decode throughput 21.1%. At quality 100, compression improves
23.6%, encode throughput 43.7%, and decode throughput 9.1%. Quality metrics are
bit-identical to all-intra at each setting. The high-motion `ed-dense-motion`
clip falls back at quality 90 and exactly retains its 4.361x ratio; at quality
100 its ratio changes from 2.765x to 2.732x (-1.2%), within the 5% corpus gate.

All fourteen Rust tests, strict Clippy, release builds, and the three Lean
proofs pass.

## Decision

Accept. Both compression gates pass with substantial speed gains and no
quality change. The 1.2% high-motion quality-100 regression is retained as a
visible outlier and motivates an entropy-aware gate rather than a lower SAD
threshold in follow-up work. A sequence container and keyframe index remain
required before this becomes a complete video format.
