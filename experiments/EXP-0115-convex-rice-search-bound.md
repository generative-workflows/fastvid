# EXP-0115 — Convex Rice search bound

Status: **ACCEPTED**

## Classification

**Version-5 exploration/exploitation** — derive a stronger exact termination
condition for the post-EXP-0113 search hotspot.

## Hypothesis

For Rice parameter `p`, aggregate unrounded cost is

```text
B(p) = n * (p + 1) + sum(value >> p).
```

Its forward difference is nondecreasing because
`sum(value >> p) - sum(value >> (p + 1))` is nonincreasing. Once `B` stops
decreasing, every later aggregate cost is at least the current value.
If `ceil(B(p)/8)` is already no smaller than the best sum of byte-rounded
lane lengths, no later parameter can beat the first minimum.

Using this bound should remain exactly equivalent to all 17 parameters and
improve geometric encode throughput at least 1.20x over EXP-0113 without
moving decode more than 5%.

## Modification

Retain the ascending per-lane exact scan. Track aggregate unrounded bits and
stop when both conditions hold:

1. aggregate bits no longer decrease; and
2. their byte ceiling is at least the current best sum of lane byte ceilings.

Keep EXP-0113's all-zero-quotient stop as a terminal fallback.

## Gate

- the existing exhaustive full-scan oracle remains green;
- control and native q90 streams are byte-identical;
- a contemporaneous balanced A/B against commit `9ed1337` reaches at least
  1.20x geometric encode and 0.95x decode throughput;
- full validation and a post-change profile pass.

## Result

The stronger bound remains exactly equivalent to the complete scan in the
existing exhaustive single-value and representative multi-lane oracle. The
control stream retains SHA-256
`9a3cf708ecdc73f9f8c15a545b41f761ad1ed844c2b8cb4db42118ce587fce37`,
and every native q90 byte, bitrate, quality metric, and maximum error remains
unchanged.

Five balanced trials alternated the exact EXP-0113 binary from commit
`9ed1337` and the candidate:

| Sample | Encode | Encode ratio | Decode ratio |
|---|---:|---:|---:|
| HDR gradient 10 | 24.443 MP/s | 1.259x | 1.032x |
| Precision motion 10 | 24.519 MP/s | 1.240x | 0.985x |
| Precision UI 12 | 28.832 MP/s | 1.387x | 0.982x |
| Precision motion 16 | 38.173 MP/s | 1.499x | 1.019x |
| **Geometric** | — | **1.3423x** | **1.0043x** |

Both performance gates pass. Relative to EXP-0110's fixed version-2 rows,
the current version-5 implementation is now 0.4065x encode and 1.2370x decode
geometrically. EXP-0113 plus EXP-0115 improve the initial version-5 encoder by
about 2.32x without changing one format byte.

A 50-repeat post-change cycle profile captured 18K samples with none lost.
The inlined version-5 tile closure accounts for 72.32%, scalar Rice emission
19.11%, fixed block 2.72%, validation 1.61%, AVX-512 `memmove` 1.37%, and
allocator self-time 0.71%. The previously distinct 68.34% exhaustive-search
symbol is gone.

The Lean specification now proves that increasing a Rice parameter cannot
increase its unary quotient, the monotonic ingredient used by the convexity
argument.

Artifacts:

- `artifacts/exp0115-convex-rice-confirm.tsv`
  (`ce5001934247d8163667203024bd2ba6ac42972911c11af68fd5cdc6df2cfa96`);
- `artifacts/exp0115-v5-encode-perf.data`
  (`4f7aff7319be9ff59d7c4a881d49885d8c9d201f55049ace8a500960ab691052`);
- `artifacts/exp0115-v5-encode-perf-report.txt`
  (`14e8e9af3d85e58030cb4970ecc987f9c827b9a005aea2b6f9a12cfcfd3e457b`).

## Decision

Accept the convex Rice lower bound. It is a broadly useful, exact
format-preserving optimization and materially advances the low-serialization
branch.

Version 5 remains below the CPU/OpenAPV encode target, so it is not promoted
over version 2. The next profiling experiment should separate residual
production, Rice selection, and zero/block candidate construction with
stable no-inline measurement wrappers or dedicated microbenchmarks. Scalar
Rice emission is now an explicit 19.11% target, but EXP-0114 shows that code
growth and complete-binary decode behavior must be included in any grouped
writer decision.

## References

- [Research 0027](../research/0027-streaming-rice-parameter-selection.md)
- [EXP-0112](EXP-0112-version5-encode-profile.md)
- [EXP-0113](EXP-0113-parallel-rice-early-termination.md)
