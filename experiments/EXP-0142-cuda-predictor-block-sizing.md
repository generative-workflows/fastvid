# EXP-0142 — CUDA predictor block sizing

Status: **REJECTED**

## Hypothesis

V5's default 256x128 access tiles have at most 128 active samples on an
antidiagonal. Launching 128 rather than 256 predictor threads and skipping the
unnecessary reconstructed-scratch clear will reduce complete q90 time by at
least 5% and cross 3 GP/s without changing bytes.

## Modification

Use 128 threads per tile for prediction and allocate reconstructed scratch
without initialization. Every dependency is written before it is read, while
tile-edge predictors use literal zero and never read scratch outside the tile.

## Test

Run full byte-identity conformance, then the same real-world 4K q90/q100
three-warm-up, ten-trial benchmark and q90 stage profile from EXP-0141.

## Result

Byte identity passed, but q90 regressed from 2.957654 to 2.996144 ms
(2.804385 to 2.768358 GP/s), a 1.3% slowdown. It missed both predeclared
gates. The presumed occupancy benefit did not offset the reduced block-level
parallelism/scheduling behavior; the scratch clear was too small to change the
outcome.

## Decision

Reject and restore 256 predictor threads plus the conservative initialized
scratch allocation. Target the measured Rice emission work instead.

## References

- [Research 0038](../research/0038-lossless-wavefront-scheduling.md)
- [EXP-0141](EXP-0141-cuda-parallel-entropy-analysis.md)
