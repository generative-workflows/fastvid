# EXP-0138 — CUDA predictor schedules

Status: **REJECTED**

## Hypothesis

Assigning one independent full-tile raster chain to each CUDA thread can beat
the antidiagonal block schedule by removing 383 block synchronizations per
default 256x128 tile. The 765 access tiles in a 4K YUV422 frame may expose
enough independent chains to occupy an L40.

## Modification

Add a diagnostic `serial` reconstruction kernel alongside the accepted
`wavefront` kernel. The serial mapping assigns one thread per tile and walks
the exact Rust clamp-gradient raster dependency chain. Expose the mapping as
`decode_v5(..., predictor="serial")` while retaining wavefront as the default.

## Test

Run the full Rust-oracle conformance matrix for both mappings: 10/12/16 bit,
q90/q100, odd edge tiles, DRAM/VRAM streams, and zero-run/Rice/fixed-block
entropy. Then measure five warmups and twenty complete calls on the real-world
3840x2160 q90 Calotes stream.

## Result

Both schedules reproduce Rust exactly. From DRAM, wavefront reached 5.658533
GP/s in 1.465822 ms while the serial mapping reached 0.541456 GP/s in
15.318689 ms. Serial was 10.45x slower. Its VRAM row was similarly slow at
0.534306 GP/s.

The removed barriers do not compensate for exposing only one long dependency
chain per thread. The wavefront kernel's 128-way diagonal parallelism is
decisive on this GPU.

## Decision

Reject scalar tile chains as the primary CUDA mapping. Keep the implementation
only as a diagnostic correctness/performance control and retain antidiagonal
wavefront as the default. Future reconstruction work should optimize the
wavefront kernel or alter predictor granularity through a separately measured
format experiment, not return to one-thread raster chains.

Raw rows are in
[`benchmarks/v5-cuda-predictor-schedules.tsv`](../benchmarks/v5-cuda-predictor-schedules.tsv).

## References

- [Research 0038](../research/0038-lossless-wavefront-scheduling.md)
- [EXP-0105](EXP-0105-predictor-wavefront-model.md)
- [EXP-0129](EXP-0129-interleaved-full-tile-predictors.md)
- [EXP-0134](EXP-0134-cuda-handoff-contract.md)
- [EXP-0137](EXP-0137-cuda-v5-decoder-baseline.md)
