# EXP-0001 — Paeth + scalar quantization + unsigned LEB128 baseline

Status: **REJECTED**

## Hypothesis

A tile-local Paeth predictor followed by scalar residual quantization and
unsigned LEB128 will establish a safe, dependency-free baseline that is:

- exactly reversible at quality 100;
- independently decodable per tile;
- parallel across tiles;
- compressive on smooth synthetic 4:2:2 material;
- simple enough to specify in Lean before optimizing.

This is informed by [FFV1 slices](../research/0001-ffv1.md) and
[PNG predictors](../research/0002-png-predictors.md). Quality reporting begins
with PSNR; [SSIM](../research/0003-ssim.md) is follow-up work.

## Modification

Implement format version zero, tile-local Paeth prediction, quality-derived
scalar quantization, zigzag mapping, and canonical unsigned LEB128. Use only
safe Rust and the standard library. Parallelize independent tile work with a
configurable worker count.

## Test

On this host, run release-mode deterministic YUV422p8 synthetic frames at
1920×1080, quality 100 and 90, with 1 and 4 threads. Record:

- encoded bytes and raw/encoded ratio;
- luma MSE, PSNR, and maximum error;
- encode/decode throughput;
- exact round-trip at quality 100;
- unit and malformed-input tests.

## Results

Host: 4-vCPU AMD EPYC-Genoa VM, 7.6 GiB RAM, no swap; Rust 1.97.1,
release profile with thin LTO. Fixture: deterministic 1920×1080 YUV422p8.
Values are single-run development measurements and are not statistical
benchmarks.

| Quality | Threads | Size | Ratio | Encode | Decode | Luma PSNR |
|---:|---:|---:|---:|---:|---:|---:|
| 100 | 1 | 4,154,345 B | 0.998× | 33.0 MP/s | 57.8 MP/s | exact |
| 100 | 4 | 4,154,345 B | 0.998× | 75.5 MP/s | 142.6 MP/s | exact |
| 90 | 1 | 4,154,151 B | 0.998× | 37.9 MP/s | 57.2 MP/s | 49.892 dB |
| 90 | 4 | 4,154,151 B | 0.998× | 73.7 MP/s | 130.6 MP/s | 49.892 dB |

All six Rust tests, strict Clippy, the release build, and the initial Lean
zigzag inverse proof passed.

## Decision

Reject the hypothesis because unsigned LEB128 consumes at least one byte per
sample and therefore cannot compress a byte-oriented source despite effective
prediction. Preserve the tiled predictor baseline, but replace the residual
stream with zero-run tokens in
[EXP-0002](EXP-0002-zero-run-tokens.md).

