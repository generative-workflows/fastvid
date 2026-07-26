# EXP-0130 — Four-parameter Rice pass

Status: **ACCEPTED**

## Classification

**Version-5 selector exploitation** — reduce residual traversals in the
39.92% exact Rice/zero-run selector hotspot after EXP-0129.

## Hypothesis

EXP-0117 improved version-5 encoding 1.1667x by deriving two adjacent Rice
quotients in one residual traversal. For a folded value, four adjacent
quotients form the exact shift chain
`q, q >> 1, q >> 2, q >> 3`. The common q90 distributions often require a
second paired traversal before the convex bound fires.

Computing parameters zero through three together should remove that second
memory traversal and repeated lane indexing. The extra shifts and counters
are independent arithmetic that a wide superscalar CPU can overlap. The
candidate should improve geometric whole-codec encode throughput at least
1.05x over EXP-0129, retain at least 0.95x decode throughput, and preserve
every stream byte.

## Modification

- evaluate up to four adjacent Rice parameters in each folded-residual pass;
- derive later quotients by shifting the first quotient;
- retain ascending candidate updates, first-minimum ties, exact per-lane byte
  rounding, convex termination, and the all-zero quotient stop;
- retain fused zero-run cost only in the parameter-zero group.

Do not change prediction, residual staging, Rice emission, entropy modes,
syntax, or output assembly.

## Test

- retain the exhaustive complete-scan Rice oracle and fused zero-run oracle;
- retain the accepted version-5 control and every native q90 byte/metric;
- run five balanced trials against the exact EXP-0129 binary;
- require at least 1.05x geometric encode and 0.95x geometric decode;
- run the full release suite, both strict Clippy configurations, formatting,
  and diff checks if accepted;
- revert the implementation if the complete-binary gate fails.

## Result

The complete-scan Rice oracle and fused zero-run oracle remain exact. The
candidate retains the accepted HDR control SHA-256
`9a3cf708ecdc73f9f8c15a545b41f761ad1ed844c2b8cb4db42118ce587fce37`,
and every native q90 encoded byte, compression ratio, encoded-stream bitrate,
PSNR component, luma block SSIM, and maximum error is unchanged.

Five balanced whole-codec trials measured:

| Sample | Candidate encode | Encode ratio | Decode ratio | Encoded bitrate |
|---|---:|---:|---:|---:|
| HDR gradient 10 | 44.769 MP/s | 1.076x | 0.996x | 333.288000 Mb/s |
| Precision motion 10 | 46.367 MP/s | 1.058x | 0.981x | 148.023112 Mb/s |
| Precision UI 12 | 51.106 MP/s | 1.120x | 1.005x | 229.381632 Mb/s |
| Precision motion 16 | 60.288 MP/s | 0.961x | 0.993x | 38.988880 Mb/s |
| **Geometric** | — | **1.0519x** | **0.9937x** | — |

The geometric gates pass. Three samples improve encode by 5.8–12.0%;
extremely sparse 16-bit motion regresses 3.9%, which remains inside the
methodology's 5% timing tolerance. Its many early parameter-zero stops cannot
benefit from the additional already-computed candidates, so this is an
important boundary for future grouping work.

The exact EXP-0129 binary has SHA-256
`fc8ba0d5444acaee395fad8e513f16556e77e7d984dfdfccfbce8f949bd03160`;
the fixed candidate binary has SHA-256
`4bf7366047a4259375b154503f53f642e5e1649f2c86dc6c9b70f783be5b4dd9`.

The profiling-feature HDR run measured 1,388.44 ms task-clock, 4.986 billion
cycles, 21.589 billion instructions, 3.055 billion branches, and 45.162
million branch misses over 30 encodes: about 4.33 instructions/cycle and
44.80 MP/s. Compared with EXP-0129's identically configured profile,
instrumented end-to-end task clock improves 1.0788x.

A 60-repeat cycle profile captured 11K samples with none lost. Exact
Rice/zero-run selection falls from 39.92% to 36.94% of self cycles. In
absolute sampled-cycle terms that is approximately 13.2% less selector work;
paired predictor staging remains stable at 16.83%. The profile therefore
confirms that fewer folded-residual traversals—not a decode or layout
artifact—produced the gain.

Artifacts:

- `artifacts/exp0130-four-parameter-rice-confirm.tsv`
  (`55c3a9e752537abed0239e2d6b6f53ec7b45b9e1f8bab90ffbef4fa5032a1589`);
- `artifacts/exp0130-stage-perf-stat.tsv`
  (`545775c8934211b3b443fdedb031f8c1581aee5e0f916a207ea737f985d3a923`);
- `artifacts/exp0130-stage-perf.data`
  (`1cb4e378dca390715da7a9c140d0e2e651fada874615691417f3371937223154`);
- `artifacts/exp0130-stage-perf-report.txt`
  (`d4c561d06282f9c8686e2892413be4cd1d478d40756fe2ecdae5ae6ed9adef66`).

All 70 library tests, motion/squeeze and binary targets, documentation tests,
normal and profiling-feature strict Clippy, shell syntax, formatting, and
diff checks pass.

## Decision

Accept four-parameter Rice cost grouping. It crosses the complete-binary gate,
preserves the low-serialization format, and reduces the dominant selector
stage without another allocation or output merge.

Relative to EXP-0110's fixed version-2 rows, version-5 geometric encode
advances from about 0.6594x to about 0.6936x. It remains non-promoted.
Further selector work should distinguish parameter-zero-dominated sparse
shards before doing extra arithmetic, or derive several exact costs from a
more compact statistic. A universal wider group would likely deepen the
observed 16-bit sparse regression.

## References

- [Research 0027](../research/0027-streaming-rice-parameter-selection.md)
- [EXP-0117](EXP-0117-paired-rice-parameter-pass.md)
- [EXP-0127](EXP-0127-post-zero-cost-profile.md)
- [EXP-0129](EXP-0129-interleaved-full-tile-predictors.md)
