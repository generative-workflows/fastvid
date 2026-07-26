# EXP-0124 — Post-direct-emission profile

Status: **ACCEPTED**

## Classification

**Measurement / branch selection** — identify the next version-5 bottleneck
after EXP-0123's accepted 1.1121x direct-emission improvement.

## Hypothesis

Direct final-span writes should reduce Rice emission and orchestration from
their combined EXP-0118 27.05% share and remove material `memmove` attribution.
Full-tile predictor/residual staging or exact Rice selection should become the
largest stage; allocation should remain below 2%.

## Modification

No codec modification. Build the disabled-by-default `profile-stages`
configuration and profile the fixed 1920x1080 10-bit HDR q90 frame with the
encode-only harness. Record hardware counters and a cycle call graph.

## Test

- collect at least 10K cycle samples with no lost samples;
- record top stage self-time and usable hardware counters;
- compare emission, copy, allocation, selection, and staging with EXP-0118;
- select the next exploitation branch from the accepted implementation.

## Result

The 30-repeat hardware-counter run measured 1,938.76 ms task-clock, 6.745
billion cycles, 22.455 billion instructions, 3.538 billion branches, and
49.011 million branch misses: approximately 3.33 instructions/cycle, 1.39%
branch misses, and 32.09 MP/s in the instrumented build. Cache references
were exposed, but cache misses again reported an unusable zero.

The 60-repeat cycle profile captured 15K samples with none lost:

| Stage/symbol | EXP-0124 self cycles | EXP-0118 |
|---|---:|---:|
| full-tile predictor, quantizer, and residual staging | 28.73% | 25.54% |
| exact parallel Rice parameter selection | 25.02% | 22.26% |
| direct Rice lane/body emission | 18.47% | 27.05% combined |
| zero-run body construction | 17.24% | 13.95% |
| fixed-block body emission | 3.52% | 3.25% |
| frame validation | 2.08% | 1.94% |
| fixed-block cost model | 1.57% | 1.22% |
| AVX-512 `memmove` | 1.50% | 1.33% |
| allocator | 0.37% | 1.54% |

Direct final-span emission reduces its former writer-plus-orchestration share
by 8.58 percentage points, about 31.7% relatively. Allocation falls by 1.17
points. The remaining `memmove` samples are attributed primarily to fixed
block rather than Rice lane assembly, so the hypothesis is supported except
for the expectation that all copy attribution would become immaterial.

Artifacts:

- `artifacts/exp0124-stage-perf-stat.tsv`
  (`a2a4201523908cdbb5c732f75572c66556ab951690902526fc6601d73c2be27e`);
- `artifacts/exp0124-stage-perf.data`
  (`235958823a8d94bb6815b6550427c566f411982fcde2f5691bf6bb62175d5ddf`);
- `artifacts/exp0124-stage-perf-report.txt`
  (`b9efec6403e9a1b2e01889b832e2e418968d25b326aba5c0b32e3460a392c498`).

## Decision

Accept the profile as evidence that the count/scan/disjoint-write
implementation removed the intended work.

Full-tile staging is now the largest stage. EXP-0119 already shows that
container bookkeeping is not material, so the next exploitation branch
should target quantizer lookup, reconstruction arithmetic, or predictor
dependency work itself. A packed quantizer/reconstruction lookup is a
testable safe-Rust branch: it trades table footprint for fewer hot-loop
operations and must be gated separately at 10/12/16 bits because the 16-bit
table is cache-sensitive. Exact Rice selection and zero-run construction
remain secondary branches.

## References

- [Research 0012](../research/0012-simd-cache-profiling.md)
- [EXP-0118](EXP-0118-post-paired-rice-profile.md)
- [EXP-0123](EXP-0123-matched-direct-emission-isolation.md)
