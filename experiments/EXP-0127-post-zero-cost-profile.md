# EXP-0127 — Post-zero-cost profile

Status: **ACCEPTED**

## Classification

**Measurement / branch selection** — identify the next version-5 bottleneck
after EXP-0126's accepted 1.1017x fused zero-run improvement.

## Hypothesis

Skipping most losing zero-run bodies should reduce the former 17.24%
zero-run symbol below 8%. Exact Rice selection will rise because it now
includes zero-run costing, but full-tile predictor/residual staging should
remain the largest stage. Allocation should remain below 1%.

## Modification

No codec modification. Profile the fixed 1920x1080 10-bit HDR q90 frame with
the `profile-stages` encode-only harness and the standard counters/cycle call
graph.

## Test

- capture at least 10K samples with none lost;
- record top self-time stages and usable counters;
- compare selector, zero-run, staging, and emission with EXP-0124;
- select the next branch from the accepted implementation.

## Result

The 30-repeat hardware-counter run measured 1,676.99 ms task-clock, 5.939
billion cycles, 20.947 billion instructions, 3.355 billion branches, and
48.045 million branch misses: approximately 3.53 instructions/cycle, 1.43%
branch misses, and 37.10 MP/s in the instrumented build. Cache references
were exposed, while cache misses again reported an unusable zero.

The 60-repeat cycle profile captured 14K samples with none lost:

| Stage/symbol | EXP-0127 self cycles | EXP-0124 |
|---|---:|---:|
| exact Rice selection plus fused zero-run cost | 36.21% | 25.02% |
| full-tile predictor and residual staging | 31.50% | 28.73% |
| shard emission/selection combined | 19.96% | not combined |
| fixed-block body emission | 4.08% | 3.52% |
| frame validation | 2.40% | 2.08% |
| AVX-512 `memmove` | 1.75% | 1.50% |
| fixed-block cost model | 1.71% | 1.57% |
| actual zero-run body construction | 1.29% | 17.24% |

The hypothesis is partly supported. Actual zero-run construction falls by
15.95 percentage points and allocator symbols disappear below the 0.3%
reporting threshold, but fused costing raises selector attribution by 11.19
points and makes it—not staging—the largest stage. The net movement remains
strongly positive, matching EXP-0126's whole-codec gain.

Artifacts:

- `artifacts/exp0127-stage-perf-stat.tsv`
  (`eae5cf204d0ed3d5545a55c6c72a2347b54534d1eeb31dccd1011292d40cff5d`);
- `artifacts/exp0127-stage-perf.data`
  (`c6678a2924e1db77372763c932d156b52a8b6ba5e6ca09af67e301d61bb17d8f`);
- `artifacts/exp0127-stage-perf-report.txt`
  (`0b7fc90a5bead50f4ecda8d58ae9a525edecbe947a5e2afa0f6954790c86ebbb`).

## Decision

Accept the profile and retain fused costing.

The next exploitation branch should separate the parameter-zero paired
traversal from later parameter pairs. Today every selector iteration executes
a per-symbol `parameter == 0` condition even though only the first pass can
charge zero-run cost. A specialized first pass can preserve identical
arithmetic and candidate order while removing that branch from every later
residual traversal. It must clear a complete-binary gate because loop
duplication changes code layout.

## References

- [Research 0012](../research/0012-simd-cache-profiling.md)
- [EXP-0124](EXP-0124-post-direct-emission-profile.md)
- [EXP-0126](EXP-0126-selector-fused-zero-run-cost.md)
