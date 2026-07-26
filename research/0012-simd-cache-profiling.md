# Safe SIMD and cache-oriented profiling in Rust

## Sources

- Rust standard library, [`std::arch`][std-arch].
- Rust nightly standard library, [`std::simd`][std-simd].
- Valgrind, [Cachegrind manual][cachegrind].

[std-arch]: https://doc.rust-lang.org/std/arch/index.html
[std-simd]: https://doc.rust-lang.org/nightly/std/simd/index.html
[cachegrind]: https://valgrind.org/docs/manual/cg-manual.html

## Findings

Rust's portable SIMD API remains nightly-only. Stable `std::arch` exposes
target intrinsics and runtime feature detection, but target-specific functions
and intrinsic calls require a carefully justified unsafe boundary. That
conflicts with Fastvid's safe-Rust default and should only be accepted after a
safe implementation and compiler auto-vectorization have been exhausted.

LLVM can auto-vectorize ordinary safe loops when:

- iteration is contiguous and bounds are simple;
- input and output aliasing is visibly impossible;
- loop-carried dependencies are separated from independent work;
- data-dependent output and branching are moved to a later pass when possible.

Cachegrind distinguishes instruction, data, and last-level cache behavior, but
it simulates caches and branch prediction rather than measuring the exact host.
Hardware counters and a sampling profiler should be preferred for final
diagnosis, with Cachegrind used for deterministic relative experiments.

Hardware-counter availability is host- and event-specific. A profiler setup
must sanity-check each requested PMU event: permission to profile one's own
process does not prove that a virtualized PMU implements every generic alias,
and a zero count is not automatically evidence of zero misses. Wall time and
sampling attribution alone must not be described as proof of cache misses.

## Current Fastvid hot-path candidates

- `ResidualAccumulator` writes folded residuals, a histogram, and a zero-run
  payload in one data-dependent pass, which inhibits vectorization and writes
  multiple working sets.
- spatial Paeth prediction has unavoidable row dependencies, but reconstructed
  tile storage and residual storage should remain contiguous.
- temporal prediction is element-independent before entropy construction and is
  the best initial auto-vectorization candidate.
- `parallel_map` creates scoped OS threads for every frame and serializes every
  completed tile through one mutex.
- decoded tiles are allocated individually and copied into final frame planes.

Optimization order should be measurement-driven: remove synchronization and
allocation overhead first, reshape temporal loops for auto-vectorization
second, and introduce explicit SIMD only with a demonstrated remaining kernel
bottleneck.

## Relevant experiments

- [EXP-0090: post-pack speed profile](../experiments/EXP-0090-post-pack-speed-profile.md)

- [EXP-0010](../experiments/EXP-0010-fast-feedback-loop.md)
- [EXP-0011](../experiments/EXP-0011-parallel-map-contention.md)
- [EXP-0015](../experiments/EXP-0015-temporal-copy-corpus-confirmation.md)
- [EXP-0021](../experiments/EXP-0021-entropy-allocation-final.md)
- [EXP-0024](../experiments/EXP-0024-quantizer-table-confirmation.md)
- [EXP-0034](../experiments/EXP-0034-perf-samply-cache-profile.md)
- [EXP-0112](../experiments/EXP-0112-version5-encode-profile.md)
- [EXP-0116](../experiments/EXP-0116-version5-stage-decomposition.md)
- [EXP-0118](../experiments/EXP-0118-post-paired-rice-profile.md)
- [EXP-0121](../experiments/EXP-0121-emission-binary-frontend-counters.md)
- [EXP-0124](../experiments/EXP-0124-post-direct-emission-profile.md)
- [EXP-0127](../experiments/EXP-0127-post-zero-cost-profile.md)
