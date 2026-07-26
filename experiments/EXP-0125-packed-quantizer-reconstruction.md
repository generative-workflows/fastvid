# EXP-0125 — Packed quantizer reconstruction table

Status: **REJECTED**

## Classification

**Version-5 staging exploration** — trade quantizer-table footprint for fewer
operations in the 28.73% predictor/residual stage isolated by EXP-0124.

## Hypothesis

The current table returns only the quantized residual; the hot loop then
multiplies it by the quantizer step and zigzags it for entropy coding. A single
64-bit entry can losslessly pack the quantized residual, reconstruction delta,
and folded residual for every supported 10/12/16-bit residual and quality.

One load plus masks/shifts should replace the hot multiply and zigzag mapping.
The candidate should improve geometric version-5 encode throughput by at least
1.05x over EXP-0123, retain at least 0.95x encode on every tested bit depth,
retain at least 0.95x decode geometrically, and emit identical streams.

## Modification

Replace each `i32` quantizer entry with a packed `u64` containing:

- signed quantized residual;
- signed reconstruction delta (`quantized * step`);
- unsigned zigzag-folded residual.

Use 18 bits per field with a signed bias for the first two. Keep the ordinary
`quantize` accessor for all existing paths and add a full-tile accessor that
returns reconstruction delta plus folded residual directly. Verify every
supported quality/residual combination against scalar arithmetic.

The table grows from four to eight bytes per residual: at 16 bits its maximum
footprint grows from 512 KiB to 1 MiB. This cost is explicit and the
per-depth performance rows are authoritative; a tiny corpus-only gain does
not justify the footprint.

## Test

- exhaustively match quantized residual, reconstruction delta, and zigzag
  mapping for every supported residual, quality, and bit depth;
- retain the accepted version-5 control hash and all native q90 bytes/metrics;
- run five balanced whole-codec trials against the exact EXP-0123 binary;
- require at least 1.05x geometric encode, 0.95x encode at every depth/sample,
  and 0.95x geometric decode;
- run the full release suite, both strict Clippy configurations, formatting,
  and diff checks.

## Result

The exhaustive test matches scalar quantization, reconstruction delta, and
zigzag mapping for every residual at all 100 qualities and 10/12/16-bit
depths. Every native q90 stream byte and bitrate remains identical.

Five balanced whole-codec trials measured:

| Sample | Candidate encode | Encode ratio | Decode ratio |
|---|---:|---:|---:|
| HDR gradient 10 | 31.276 MP/s | 0.966x | 1.013x |
| Precision motion 10 | 33.655 MP/s | 0.990x | 1.039x |
| Precision UI 12 | 35.777 MP/s | 0.951x | 0.990x |
| Precision motion 16 | 48.967 MP/s | 0.996x | 0.961x |
| **Geometric** | — | **0.9756x** | **1.0006x** |

The candidate regresses geometric encode by 2.44% and no sample improves
materially. It fails the 1.05x gate; decode is unchanged within tolerance.
The result indicates that table construction and the doubled cache footprint
outweigh removal of the hot multiply and zigzag arithmetic.

The fixed EXP-0123 binary has SHA-256
`d828b8a79f94194baa3f1593a9acf67a6a4f915dd443b8d7120fc088c06291dc`;
the candidate has SHA-256
`bf4d9a60f0215662d4bb1ae4853fa3fe8516421efd54eb9a5810f2ee03d8ddd5`.
The balanced artifact is
`artifacts/exp0125-packed-quantizer-confirm.tsv`
(`3449243c7e7f8197caa50be600ccfba6e3a5836773fe6bebb6c573414f894be9`).
The exhaustive packed-table test and strict normal Clippy pass.

## Decision

Reject and revert the packed table. A 1 MiB 16-bit quantizer table is not
justified by a measured encode regression.

The staging path needs a smaller-state transformation. Preserve the existing
four-byte table and arithmetic; subsequent work should investigate predictor
dependency reduction, a narrower auxiliary table only for common depths, or
zero-run modeling rather than widening every residual entry.

## References

- [Research 0012](../research/0012-simd-cache-profiling.md)
- [EXP-0119](EXP-0119-trusted-length-residual-staging.md)
- [EXP-0124](EXP-0124-post-direct-emission-profile.md)
