# EXP-0117 — Paired Rice parameter pass

Status: **ACCEPTED**

## Classification

**Version-5 speed exploitation** — reduce the 31.59% exact-selection hotspot
isolated by EXP-0116 without changing the bounded-shard format.

## Hypothesis

EXP-0115 commonly has to evaluate at least adjacent Rice parameters before
its convex bound can terminate. For a folded value, the quotient for
parameter `p + 1` is exactly `(value >> p) >> 1`. Accumulating the lane costs
for `p` and `p + 1` in one residual traversal should avoid a second traversal
and repeated lane-index work while preserving the existing ascending
candidate order, first-minimum tie rule, and exact stopping proof.

The paired pass should improve geometric version-5 encode throughput by at
least 1.08x over EXP-0115, retain at least 0.95x decode throughput, and emit
identical bytes and quality metrics.

## Modification

Evaluate parameters in adjacent pairs. Traverse the folded shard once to
accumulate separate four-lane bit counts and quotient sums for both
parameters. Apply the existing candidate update and termination conditions
to the lower parameter first; only evaluate the already-computed upper
candidate if the lower one does not terminate the search. Evaluate the final
unpaired maximum parameter normally.

Do not change Rice emission, shard selection, prediction, reconstruction, or
the bitstream syntax.

## Test

- keep the exhaustive full-scan oracle green for every individual folded
  value through 131,070 and the existing representative multi-lane shards;
- retain the EXP-0110 version-5 control SHA-256 and exact encoded bytes on
  every native q90 sample;
- run five balanced alternating trials against the exact EXP-0115 binary;
- require at least 1.08x geometric encode and 0.95x geometric decode
  throughput;
- retain identical bitrate, PSNR, SSIM, and maximum error;
- run the full release suite, normal and profiling-feature strict Clippy,
  formatting, and diff checks.

## Result

The paired selector remains exactly equivalent to the complete scan for every
individual folded value from 0 through 131,070 and for the existing
representative multi-lane shards. The candidate and EXP-0115 binaries both
emit the accepted control SHA-256
`9a3cf708ecdc73f9f8c15a545b41f761ad1ed844c2b8cb4db42118ce587fce37`.

Five balanced trials alternated the exact EXP-0115 binary and the candidate:

| Sample | Candidate encode | Encode ratio | Decode ratio |
|---|---:|---:|---:|
| HDR gradient 10 | 29.094 MP/s | 1.174x | 0.988x |
| Precision motion 10 | 30.758 MP/s | 1.200x | 0.998x |
| Precision UI 12 | 33.947 MP/s | 1.154x | 1.004x |
| Precision motion 16 | 43.991 MP/s | 1.140x | 0.993x |
| **Geometric** | — | **1.1667x** | **0.9956x** |

The 1.08x encode and 0.95x decode gates both pass. Every encoded byte,
compression ratio, encoded-stream bitrate, PSNR component, luma block SSIM,
and maximum error remains identical between variants. Relative to EXP-0110's
fixed version-2 rows, the version-5 implementation has advanced from 0.4065x
to approximately 0.4742x geometric encode throughput without changing its
format, rate, quality, decode, or access point.

The fixed reference binary has SHA-256
`da87dac8cb0bcbc14053edcccc1ef69914b71fa12723fecd0da53bc4da64a5fc`;
the candidate binary has SHA-256
`df4818b6b296103862277c50e1245703db7c9e2ee24d4e133fe4541d8659dcc6`.
The balanced artifact is
`artifacts/exp0117-paired-rice-confirm.tsv`
(`63e9b42c740885e62ad14d28f6fccaad7e800d526d2ace49705c98a9daedc1e1`);
the candidate control is
`artifacts/exp0117-version5-control.fvid` (the accepted stream hash above).

All 67 library tests, motion/squeeze tests, binary and documentation tests,
normal and profiling-feature strict Clippy, formatting, and diff checks pass.

## Decision

Accept the paired pass. It is a format-preserving optimization across all
tested bit depths and content classes, and its gain is substantially above
the timing tolerance.

Version 5 remains below the CPU/OpenAPV encode target, so it stays an
experimental low-serialization branch rather than replacing the preserved
version-2 speed slot. The next exploitation branch should re-profile the
paired selector and then target whichever of scalar Rice emission,
predictor/residual staging, or zero-run construction has become dominant.

## References

- [Research 0027](../research/0027-streaming-rice-parameter-selection.md)
- [EXP-0115](EXP-0115-convex-rice-search-bound.md)
- [EXP-0116](EXP-0116-version5-stage-decomposition.md)
