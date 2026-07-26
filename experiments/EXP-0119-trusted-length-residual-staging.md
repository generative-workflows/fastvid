# EXP-0119 — Trusted-length residual staging

Status: **REJECTED**

## Classification

**Version-5 speed exploitation** — reduce the 25.54% full-tile staging
hotspot identified by EXP-0118.

## Hypothesis

The optimized profile build's full-tile loop retains a capacity comparison
and vector-length update around every `folded.push`, even though the vector
has exact tile-area capacity. Extending from a trusted-length
`Map<Zip<...>>` once per source row should let `Vec` reserve and write the
known row span without per-sample growth bookkeeping.

This safe, format-preserving loop shape should improve geometric version-5
encode throughput by at least 1.05x over EXP-0117, retain at least 0.95x
decode throughput, and emit identical bytes and quality metrics.

## Modification

Retain exact full-tile allocation, clamp-gradient prediction, table
quantization, reconstruction, and zigzag mapping. Replace the inner
`folded.push` loop with `folded.extend` over the exact-size zip of the source
row and reconstruction row. Carry `left` and `upper_left` through the mapping
closure and return each folded residual in raster order.

Do not change entropy coding, predictor decisions, reconstruction arithmetic,
or syntax.

## Test

- retain the exhaustive codec suite and accepted version-5 control hash;
- run five balanced alternating trials against the exact EXP-0117 binary;
- require at least 1.05x geometric encode and 0.95x geometric decode
  throughput;
- require identical bytes, bitrate, PSNR, SSIM, and maximum error;
- inspect optimized code to confirm that the per-sample vector-growth path is
  removed;
- pass normal and profiling-feature strict Clippy, formatting, and diff
  checks.

## Result

Optimized assembly confirms the intended mechanical change. The predecessor
per-sample loop compared vector length with capacity and retained a growth
path around every folded residual. The trusted-length candidate performs one
capacity check per row and writes each mapped residual directly into the
reserved span.

Five balanced trials nevertheless show only a small whole-codec change:

| Sample | Candidate encode | Encode ratio | Decode ratio |
|---|---:|---:|---:|
| HDR gradient 10 | 29.487 MP/s | 0.994x | 1.025x |
| Precision motion 10 | 31.356 MP/s | 1.031x | 1.020x |
| Precision UI 12 | 33.098 MP/s | 1.017x | 1.010x |
| Precision motion 16 | 43.942 MP/s | 1.008x | 0.992x |
| **Geometric** | — | **1.0124x** | **1.0117x** |

Every encoded byte and bitrate remains exact, but the 1.05x encode gate
fails. HDR encode is slightly slower, while the other changes are within the
methodology's timing tolerance.

The fixed EXP-0117 binary has SHA-256
`df4818b6b296103862277c50e1245703db7c9e2ee24d4e133fe4541d8659dcc6`;
the candidate binary has SHA-256
`c768e54a5953a1e8e1a34b3cf3a74dadd8615ebd56be901d9a979713167aa53c`.
The balanced artifact is
`artifacts/exp0119-trusted-extend-confirm.tsv`
(`291b96cda1ff9dc13d52ef3cb5f508f556be0f93016513291815cbca452242a0`).
The targeted version-5 test and strict normal Clippy pass.

## Decision

Reject and revert the trusted-length loop. It improves generated bookkeeping
but not enough end-to-end performance to justify a production change, and
the tiny result is plausibly corpus/compiler bound.

The 25.54% staging share is dominated by useful predictor, quantizer lookup,
reconstruction, and residual stores rather than `Vec::push` bookkeeping.
Subsequent staging work should change one of those operations or its data
dependencies, not merely the container API.

## References

- [Research 0012](../research/0012-simd-cache-profiling.md)
- [EXP-0117](EXP-0117-paired-rice-parameter-pass.md)
- [EXP-0118](EXP-0118-post-paired-rice-profile.md)
