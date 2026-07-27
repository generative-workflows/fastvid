# EXP-0137 — CUDA v5 decoder baseline

Status: **ACCEPTED**

## Hypothesis

Version 5's bounded entropy shards and full-tile antidiagonal dependency graph
can be decoded directly on CUDA without changing the Rust bitstream, and a
correct first implementation can exceed 2 GP/s on real-world 4K content.

## Modification

Add a PyTorch C++/CUDA extension that:

1. accepts a one-dimensional uint8 v5 stream tensor in DRAM or VRAM;
2. validates the canonical header, tile directory, and shard boundaries;
3. decodes zero-run, four-lane Rice, and fixed-block shards on CUDA;
4. reconstructs each access tile using exact antidiagonal clamp-gradient
   prediction; and
5. returns contiguous CUDA uint16 Y/Cb/Cr tensors.

The extension preserves the v5 format and uses the scalar Rust encoder and
decoder as its oracle. No Rust codec behavior changed.

## Test

- Compile for the L40's SM 8.9 target with CUDA 12.8 and PyTorch 2.8.
- Compare GPU output exactly with Rust for 10/12/16-bit, q90/q100, odd edge
  tiles, DRAM/VRAM input, and all three selected shard mode families.
- Convert the real-world 4K Calotes corpus sample to 10-bit, encode q90/q100
  with Rust v5, and run five warmups plus twenty complete-call trials.
- Measure Rust PSNR/full block-SSIM/error and FFmpeg 8.1.2 XPSNR.

## Result

Every conformance case passed. On the 3840x2160 real-world frame, q90 produced
2,955,209 bytes from 33,177,600 raw bytes (11.226820x), 51.941953 dB luma
PSNR, 57.9855 dB luma XPSNR, 0.99577983 block SSIM, and maximum error 4.

Complete-call median decode was 1.459942 ms / 5.681322 GP/s from DRAM and
1.690662 ms / 4.906007 GP/s from VRAM. Q100 was byte-exact and decoded at
4.443809 GP/s from DRAM and 3.043572 GP/s from VRAM.

The q90 point exceeds the three initial numeric goals simultaneously on this
sample. It is not a corpus-wide claim and does not complete the GPU reference:
encoding, video/frame-range APIs, tiled public APIs, 4:4:4, and device-side
stream preparation remain.

## Decision

Accept the CUDA decoder as the first correct GPU implementation and baseline.
Retain host validation for safety, but prioritize eliminating the VRAM-to-host
whole-stream copy and separating stage timings. Then implement the byte-
identical v5 encoder before considering format changes.

Detailed rows and environment are in
[`benchmarks/v5-cuda-decode-baseline.md`](../benchmarks/v5-cuda-decode-baseline.md).

## References

- [Research 0038](../research/0038-lossless-wavefront-scheduling.md)
- [Research 0042](../research/0042-gpu-variable-output-assembly.md)
- [Research 0043](../research/0043-xpsnr-quality-metric.md)
- [EXP-0134](EXP-0134-cuda-handoff-contract.md)
- [EXP-0135](EXP-0135-cpu-gpu-baseline.md)
- [EXP-0136](EXP-0136-corpus-v3-native-2k-4k.md)
