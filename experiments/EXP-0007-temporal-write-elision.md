# EXP-0007 — Elide temporal-tile reconstruction writes

Status: **ACCEPTED**

## Hypothesis

Temporal tile encoding can improve one-thread encode throughput by at least 3%
on the corpus-v1 GOP-12 video subset by avoiding a tile-sized reconstruction
allocation, spatial-neighbor loads, and reconstruction writes that are only
needed by Paeth prediction. Encoded bytes and reconstruction quality must be
bit-identical to EXP-0005, with no decode throughput regression above 5%.

This isolates the speed effect observed while testing rejected
[EXP-0006](EXP-0006-tile-local-temporal-gating.md). It also follows the
[temporal DPCM review](../research/0007-temporal-dpcm-gating.md) and the
data-oriented memory-access requirement in `INSTRUCTIONS.md`; no bitstream
or decoder behavior changes.

## Modification

- Allocate the tile reconstruction buffer only for spatial prediction.
- On temporal tiles, read the co-located reference sample directly and omit
  unused left/above/upper-left loads and reconstructed-sample stores.
- Retain the accepted frame-level temporal gate, entropy selection, tile
  scheduling, and all public APIs unchanged.

## Test

Run the standard corpus-v1 video subset at qualities 90 and 100, GOP 12, with
one and four threads. Compare against EXP-0005 artifacts. Require exact
per-sample encoded byte counts, PSNR, SSIM, and maximum error. Accept when
one-thread arithmetic-mean encode MP/s improves by at least 3% at both quality
levels and decode throughput does not regress by more than 5%.

## Results

Host: 4-vCPU AMD EPYC-Genoa VM, 7.6 GiB RAM, Rust 1.97.1. Single-trial
development measurements use the same inputs and GOP as EXP-0005; the standard
five-trial harness remains required for release claims.

| Quality | Mode | Encode 1T | Decode 1T | Encode 4T | Decode 4T |
|---:|---|---:|---:|---:|---:|
| 90 | EXP-0005 baseline | 40.25 MP/s | 72.43 MP/s | 135.79 MP/s | 230.12 MP/s |
| 90 | write elision | 47.04 MP/s | 75.33 MP/s | 161.82 MP/s | 251.29 MP/s |
| 100 | EXP-0005 baseline | 35.13 MP/s | 61.50 MP/s | 116.72 MP/s | 199.26 MP/s |
| 100 | write elision | 38.67 MP/s | 62.48 MP/s | 135.64 MP/s | 209.84 MP/s |

One-thread encode throughput improved 16.8% at quality 90 and 10.1% at quality
100. Four-thread encode throughput improved 19.2% and 16.2%. Decode variation
was positive at all four points, from 1.6% to 9.2%.

Every per-sample encoded byte count is identical to EXP-0005. Aggregate
bitrates remain 127.56 Mb/s at quality 90 and 206.73 Mb/s at quality 100;
PSNR, SSIM, and maximum errors are unchanged. All fourteen Rust tests, strict
Clippy, release builds, and the three Lean proofs pass.

## Decision

Accept. Both one-thread speed gates pass by wide margins with bit-identical
output and no decode regression. The specialized temporal loop also makes the
absence of spatial reconstruction state explicit, reducing allocation and
memory traffic without changing the format.
