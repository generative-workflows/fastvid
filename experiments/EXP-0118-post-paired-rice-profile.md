# EXP-0118 — Post-paired Rice profile

Status: **ACCEPTED**

## Classification

**Measurement / branch selection** — identify the next version-5 bottleneck
after EXP-0117's exact 1.1667x encode improvement.

## Hypothesis

Pairing adjacent parameters should reduce exact Rice selection below its
EXP-0116 31.59% share. Scalar Rice emission or full-tile
predictor/quantizer/residual staging should become the largest individually
attributable stage, while allocation should remain below 2%.

## Modification

No codec modification. Build the EXP-0116 `profile-stages` configuration and
profile the fixed 1920x1080 10-bit HDR q90 frame with the encode-only harness.
Record hardware counters and a cycle call graph without profiling input
parsing or frame construction.

## Test

- collect at least 10K cycle samples with no lost samples;
- record top self-time stages and available hardware counters;
- compare stage shares with EXP-0116;
- choose the next exploitation branch from the new profile.

## Result

The 30-repeat hardware-counter run measured 2,096.69 ms task-clock, 7.416
billion cycles, 26.062 billion instructions, 3.903 billion branches, and
49.008 million branch misses: approximately 3.51 instructions/cycle, 1.26%
branch misses, and 29.67 MP/s in the instrumented build. Cache references
were exposed, but cache misses again reported an unusable zero.

The 60-repeat cycle profile captured 17K samples with none lost:

| Stage/symbol | EXP-0118 self cycles | EXP-0116 |
|---|---:|---:|
| full-tile predictor, quantizer, and residual staging | 25.54% | 23.20% |
| scalar Rice bit emission | 23.60% | 21.33% |
| exact parallel Rice parameter selection | 22.26% | 31.59% |
| zero-run body construction | 13.95% | 12.41% |
| Rice lane/body orchestration | 3.45% | 2.01% |
| fixed-block body emission | 3.25% | 2.45% |
| frame validation | 1.94% | 1.59% |
| AVX-512 `memmove` | 1.33% | 1.57% |
| fixed-block cost model | 1.22% | 1.07% |
| allocator/reallocator | 1.54% | 1.05% |

EXP-0117 reduces selector share by 9.33 percentage points and makes full-tile
staging the largest individually attributed stage. Scalar Rice emission is a
close second. The profile supports the measured speedup rather than exposing
a displaced regression; allocator time remains minor.

Artifacts:

- `artifacts/exp0118-stage-perf-stat.tsv`
  (`8a9d857a2401174c266553bd6c65a5c80f9517b5dccf2826ac228e2dcc2492de`);
- `artifacts/exp0118-stage-perf.data`
  (`dd0f7670461021444e891ab5cb2f30fafb6c0621b879b0beca78833546fb2ca3`);
- `artifacts/exp0118-stage-perf-report.txt`
  (`23c1c38c1df16b3bd88ce83eed98b36c87adf9a1fa3b7a2eae9560d8390f58aa`).

## Decision

Accept the profile as branch-selection evidence. The next exploitation branch
should target full-tile prediction/residual staging while preserving the
format and bounded-state layout. Because clamp-gradient prediction is causal,
the first candidate should reduce per-sample bookkeeping or redundant
addressing within the scalar wavefront rather than claim unconstrained SIMD.

Rice emission remains a parallel exploration branch, but EXP-0114's complete
binary regression means it must use a balanced whole-codec gate rather than a
writer-only microbenchmark.

## References

- [Research 0012](../research/0012-simd-cache-profiling.md)
- [EXP-0116](EXP-0116-version5-stage-decomposition.md)
- [EXP-0117](EXP-0117-paired-rice-parameter-pass.md)
