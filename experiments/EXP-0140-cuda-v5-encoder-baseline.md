# EXP-0140 — CUDA v5 encoder baseline

Status: **ACCEPTED**

## Hypothesis

The complete Rust v5 encoder can be reproduced on CUDA without changing one
bit, providing a measurable baseline before any performance optimization.

## Modification

Add a PyTorch CUDA encoder for one uint16 grayscale or planar YUV 4:2:2 frame.
It performs full-tile antidiagonal prediction/quantization, exact per-shard
zero-run/Rice/fixed-block selection, compact host offset assembly, and
disjoint GPU emission. It returns a CUDA uint8 v5 stream.

## Test

Compare complete CUDA and Rust streams at 10/12/16-bit, q90/q100, odd edge
tiles, and inputs selecting all three entropy families. Then benchmark the
real-world 3840x2160 10-bit Calotes frame with three warm-ups and ten trials.

## Result

All streams matched byte-for-byte. Q90 encoded in 60.366241 ms (0.137401
GP/s); q100 encoded in 60.479805 ms (0.137143 GP/s) and remained exact.
Profiling attributed 57.842 ms, or 96.1% of CUDA time, to the one-thread-per-
shard entropy analyzer. Prediction was 1.036 ms and emission 1.194 ms.

The extension SHA-256 was
`3065221b85b4fdb5bf3bcd17b3c0169c3507bb1a838d37c41519bc86e8ff0807`;
the Rust oracle SHA-256 was
`224782496805cc15ee86290515010804b613ea4375a96a986accd86a7e654a69`.

## Decision

Accept as the first correct encoder baseline. The profile makes exact entropy
analysis the next exploitation target; no format or quality change is
justified.

## References

- [Research 0038](../research/0038-lossless-wavefront-scheduling.md)
- [Research 0042](../research/0042-gpu-variable-output-assembly.md)
- [EXP-0139](EXP-0139-cuda-encoding-feedback-baseline.md)
