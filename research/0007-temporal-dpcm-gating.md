# 0007 — Temporal DPCM and block-adaptive gating

Sources:

- R. F. Rice and J. R. Plaunt, “Adaptive Variable-Length Coding for Efficient
  Compression of Spacecraft Television Data,” 1971:
  https://ntrs.nasa.gov/citations/19720033735
- NASA, “The CCSDS Lossless Data Compression Recommendation for Space
  Applications,” 2001:
  https://ntrs.nasa.gov/api/citations/20010073033/downloads/20010073033.pdf

Terms: both records are publicly accessible US-government research. Fastvid
uses the general predictive and block-adaptive architecture and copies no
implementation.

## Findings

Rice and Plaunt combine sample prediction with simple adaptive codes and
report performance close to difference entropy across a broad entropy range.
The CCSDS architecture preprocesses samples into residual-like nonnegative
symbols, evaluates coding options independently per block, and transmits a
selector. It includes a no-compression escape because a predictor/coder that
fits one source can expand another.

Video frames with modest motion have strong co-located temporal correlation,
but direct frame differencing fails under large motion or scene changes. A
cheap activity gate before entropy coding follows the same adaptive principle:
use the temporal predictor only when mean luma difference indicates that it is
likely to reduce residual energy.

## Fastvid implications

1. Use only the previous reconstructed frame, preventing lossy drift.
2. Preserve tile independence within a predicted frame.
3. Bound random-access cost with a short keyframe interval.
4. Gate temporal prediction with a frame luma-SAD prepass and retain spatial
   coding for high motion.
5. Treat motion compensation as separate future research with explicit patent
   review.

## Relevant experiments

- [EXP-0005](../experiments/EXP-0005-gated-temporal-prediction.md) tests
  previous-frame prediction and the high-motion fallback on corpus v1.
- [EXP-0007](../experiments/EXP-0007-temporal-write-elision.md) specializes
  the selected temporal encoder path to remove unused spatial state.
