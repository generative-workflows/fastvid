# EXP-0094 — Strength-reduced quantizer

Status: **REJECTED**

## Classification

**Kernel exploration** — exchange the profiled dependent quantizer-table
load for exact multiply-and-shift arithmetic.

## Hypothesis

A frame-local `StrengthReducedU32` divider can compute the existing rounded
quantizer exactly without a hardware divide or residual-table load. Removing
that dependent memory access should improve matched q90 one-thread encode by
at least 2%, particularly when the causal predictor limits instruction-level
parallelism.

## Modification

1. Precompute one safe unsigned strength-reduced divider per frame.
2. Quantize the residual magnitude using that divider, then restore its sign.
3. Remove the residual lookup table from the candidate.
4. Preserve rounding, prediction, entropy syntax, decoder, and all stream
   decisions exactly.

The candidate uses `strength_reduce` 0.2.4, licensed MIT OR Apache-2.0. It
adds no unsafe code or C/C++ dependency.

## Gate

- exhaustive equality with scalar quantization for every residual, quality,
  and 10/12/16-bit depth;
- byte- and metric-identical focused q90/q100 streams;
- at least 2% matched q90 one-thread encode improvement;
- decode no worse than 5%;
- strict Clippy, formatting, and release tests pass; and
- no slow-tier run unless the focused gate passes.

## Result

The candidate used `strength_reduce` 0.2.4 with no unsafe code. Strict
release Clippy and formatting passed, and exhaustive tests matched scalar
quantization for every residual and every quality at 10, 12, and 16 bits.

A balanced two-trial q90 one-thread screen measured:

| Depth | Baseline encode | Candidate encode | Delta | Decode delta | Bytes |
|---:|---:|---:|---:|---:|---:|
| 10-bit | 71.462 MP/s | 59.270 MP/s | -17.060% | -0.194% | identical |
| 16-bit | 68.676 MP/s | 57.597 MP/s | -16.133% | -2.112% | identical |
| geometric aggregate | 70.055 MP/s | 58.428 MP/s | -16.598% | -1.157% | identical |

PSNR and block SSIM were identical. Exact reciprocal arithmetic is
substantially slower than the accepted table on both table sizes, so the
candidate failed the fast gate and no confirmation or slow-tier run was
performed.

Artifacts:

- focused raw results:
  `artifacts/exp0094-strength-reduced-smoke.tsv`
  (`2c9d2d77d3941c08871c524601d0176329e976280f1d67acad5150d130d8654a`);
- candidate binary:
  `artifacts/frontier/fastvid-speed-exp0094-strength-reduced`
  (`0f2ef628f34fcf4d5d30c4e76511e0561354804b87906876d93fd641cbfbb6da`).

## Decision

Reject the arithmetic candidate, remove the dependency, and retain the
accepted lookup table. EXP-0093 and this experiment jointly show that the
profiled load cannot be improved by merely dropping its bounds check or
replacing it with scalar reciprocal arithmetic. Future work should overlap
or restructure the causal loop instead of revisiting quantizer mechanics.

## References

- [Research 0035](../research/0035-runtime-invariant-integer-division.md)
- [EXP-0027](EXP-0027-high-bit-quantizer-table.md)
- [EXP-0090](EXP-0090-post-pack-speed-profile.md)
- [EXP-0093](EXP-0093-proven-quantizer-lookup.md)
