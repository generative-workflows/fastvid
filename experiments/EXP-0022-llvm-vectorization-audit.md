# EXP-0022 — LLVM vectorization audit

Status: **ACCEPTED**

## Hypothesis

LLVM optimization remarks and native-target assembly can distinguish loops
already benefiting from SIMD from residual loops blocked by histogram updates,
data-dependent token output, or spatial prediction dependencies.

## Modification

No codec modification. Build the release library for the native EPYC target
with LLVM loop-vectorization remarks and emitted assembly.

## Test

1. Record successful/missed vectorization remarks for codec loops.
2. Search native assembly for AVX-family packed operations.
3. Map missed hot loops back to source and choose the next restructuring
   experiment.

## Acceptance criteria

- Evidence is tied to source loops or clearly marked inconclusive.
- No claim of hardware cache misses is made without hardware counters.
- The audit produces a concrete next SIMD/layout target.

## Results

Rust 1.97.1/LLVM 22.1.6 was built with `-C target-cpu=native` on the AVX-512
capable EPYC host. The emitted library assembly contains AVX-width moves and
standard-library `memcpy` calls, including the accepted temporal bulk-copy
path, but the temporal residual loop remains scalar. Its inner loop contains
per-sample signed `idivl`, scalar histogram increments, and data-dependent
zero-run length branches; no packed subtract/quantize arithmetic is emitted.

LLVM did not emit source-linked loop remarks through this rustc invocation, so
the assembly evidence is used directly and claims are limited to the identified
temporal loop. This is not evidence about hardware cache misses.

## Conclusion

Accepted. Explicit SIMD would still be blocked by scalar division and
scatter-like histogram updates. The next experiment replaces millions of
per-sample divisions with a small L1-resident quantization lookup table before
attempting further loop separation.


## References

- [Research 0012](../research/0012-simd-cache-profiling.md)
- [EXP-0021](EXP-0021-entropy-allocation-final.md)
