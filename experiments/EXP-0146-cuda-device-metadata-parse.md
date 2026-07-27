# EXP-0146 — CUDA device-side v5 metadata parsing

Status: **ACCEPTED**

## Hypothesis

VRAM decode currently copies the entire encoded stream to pageable host memory
to validate and construct metadata, then uploads shard and tile metadata. On
the real-world Calotes 4K q90 control, entropy plus reconstruction kernels take
about 1.0 ms while the complete VRAM call takes 2.38 ms. Copying only the
32-byte header to host and validating the directory/shard records on CUDA will
improve complete VRAM decode throughput by at least 30%.

## Modification

Keep fixed-header parsing on the host for output allocation. For CUDA-resident
streams, construct canonical expected tile geometry from that header, then
launch one device parser thread per tile to validate directory continuity and
bounded-shard headers while writing decode metadata directly in VRAM. Retain
the existing host parser for DRAM inputs.

## Test

Require pixel identity with Rust and the existing CUDA path across q90/q100,
10/12/16-bit, grayscale/YUV 4:2:2, odd edge tiles, all entropy modes, and the
malformed-stream rejection suite. Accept only if Calotes 4K q90 VRAM
complete-call throughput improves by at least 30% and DRAM throughput regresses
by no more than 5%. Also report the two representative 1080p camera controls.

## Result

The complete conformance suite remained pixel-identical across 10/12/16-bit,
q90/q100, grayscale/YUV 4:2:2, odd edge tiles, both reconstruction schedules,
and all entropy modes. Five CUDA-resident mutations covering noncanonical
directory fields, broken payload continuity, an unknown shard mode,
truncation, and trailing bytes were rejected.

Against the EXP-0145 full-corpus control epoch, Calotes 4K q90 VRAM decode
improved from 2.316797 to 6.548573 GP/s (+182.7%). DRAM changed from 3.849083
to 3.932471 GP/s (+2.2%). In a 20-trial control run, camera-pontegana and
camera-cholla VRAM decode measured 3.025980 and 3.225079 GP/s. Profiling reduced
device-to-host traffic from a full encoded stream plus status to two tiny
transfers totaling 2.881 us; the device metadata parser itself took 6.944 us.

## Decision

Accept. VRAM improvement exceeds the 30% gate, DRAM does not regress, malformed
input handling remains explicit, and decoded pixels remain identical. Refresh
the full corpus before selecting the next kernel bottleneck.

## References

- [EXP-0137](EXP-0137-cuda-v5-decoder.md)
- [EXP-0139](EXP-0139-cuda-feedback-loop.md)
- [CUDA handoff architecture](../specs/architecture.md#cuda-version-5-handoff)
