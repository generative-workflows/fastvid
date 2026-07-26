# EXP-0116 — Version-5 stage decomposition

Status: **ACCEPTED**

## Classification

**Measurement / exploration** — resolve the 72.32% inlined tile closure after
EXP-0115.

## Hypothesis

With profiling-only no-inline boundaries, full-tile predictor/quantizer and
residual staging will be the largest remaining stage, followed by Rice
emission and selection. Zero-run and block candidate construction should each
remain below 10%; allocation should remain below 2%.

## Modification

Add a disabled-by-default `profile-stages` Cargo feature. Under that feature
only, keep the version-5 tile, shard, zero-run, Rice selector/emitter, and
block-cost functions out of line so `perf` can attribute cycles. Normal
release builds retain their existing inlining decisions.

## Gate

- profiled and ordinary builds emit the exact EXP-0110 control hash;
- use an encode-only repeated run with at least 10K cycles samples and no
  lost samples;
- record top stage self-time and hardware-counter availability;
- normal and profiling builds pass strict Clippy.

## Result

Both ordinary and `profile-stages` builds emit the exact accepted control
SHA-256
`9a3cf708ecdc73f9f8c15a545b41f761ad1ed844c2b8cb4db42118ce587fce37`.
The feature changes attribution boundaries only.

The encode-only 30-repeat hardware-counter run measured 2,458.83 ms
task-clock, 8.620 billion cycles, 26.947 billion instructions, 3.906 billion
branches, and 50.005 million branch misses: about 3.13 instructions/cycle,
1.28% branch misses, and 25.30 MP/s in the instrumented build. Cache
references were exposed but cache misses again reported an unusable zero.

A 60-repeat cycle profile captured 21K samples with none lost:

| Stage/symbol | Self cycles |
|---|---:|
| exact parallel Rice parameter selection | 31.59% |
| full-tile predictor, quantizer, and residual staging | 23.20% |
| scalar Rice bit emission | 21.33% |
| zero-run body construction | 12.41% |
| fixed-block body emission | 2.45% |
| Rice lane/body orchestration | 2.01% |
| frame validation | 1.59% |
| AVX-512 `memmove` | 1.57% |
| fixed-block cost model | 1.07% |
| allocator/reallocator | 1.05% |

The hypothesis is only partly supported: prediction is the second-largest
stage, but exact Rice selection remains the largest even after EXP-0115.
Zero-run construction is also materially larger than expected. Allocation
remains minor.

Artifacts:

- `artifacts/exp0116-stage-perf-stat.tsv`
  (`bf63ba319da1b36b409d3615e11ced78114300ec1b5b66cca591eeb370c9cec4`);
- `artifacts/exp0116-stage-perf.data`
  (`a5c5990a0fae82d5509ee47965d5db4d24a305fc113628e28ac701f946a8344d`);
- `artifacts/exp0116-stage-perf-report.txt`
  (`0c3bd1e5e8ce472466a74412a74f79b8bf1f9021b3f77c91cb6e634cceb47850`).

## Decision

Accept the stage decomposition and retain the disabled-by-default feature as
profiling infrastructure.

The next exploitation branch should evaluate adjacent Rice parameters in one
residual traversal. EXP-0115 commonly needs parameters 0 and 1 before its
convex proof can stop; computing both quotient/lane sums together preserves
the exact decision while avoiding a second array traversal and repeated lane
indexing. A separate exploration branch should model zero-run cost during
residual staging, but EXP-0111 warns against adding a new scan merely to avoid
body construction.

## References

- [Research 0012](../research/0012-simd-cache-profiling.md)
- [EXP-0112](EXP-0112-version5-encode-profile.md)
- [EXP-0115](EXP-0115-convex-rice-search-bound.md)
