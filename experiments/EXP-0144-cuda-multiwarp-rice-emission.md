# EXP-0144 — CUDA multi-warp Rice emission

Status: **REJECTED**

## Hypothesis

The full-corpus panel shows that high-entropy 1080p inputs are limited by Rice
emission: camera-pontegana spends 705 us in emission versus 69 us for
camera-cholla at the same dimensions. Computing each 32-symbol group's exact
bit offset during analysis and assigning four warps to each Rice lane will
reduce camera-pontegana q90 emission by at least 30%.

## Modification

Retain a device-only table of 32-symbol Rice group offsets from exact analysis.
During emission, use sixteen warps per shard (four per lane), allowing four
independent groups in each lane to write concurrently. Preserve the v5 lane
layout and aligned atomic bit writes exactly.

## Test

Require whole-stream CUDA/Rust byte identity across the conformance suite.
Compare q90 complete-call and stage times on camera-pontegana (slow 1080p),
camera-cholla (fast 1080p), and Calotes (real-world 4K). Accept only if the slow
sample's emission falls by at least 30% and neither control regresses by more
than 5% in complete-call throughput.

## Result

Whole-stream conformance remained byte-exact, but the hypothesis was false.
Camera-pontegana emission was effectively unchanged at 702.692 us versus the
705.375 us baseline, while analysis increased from 176.672 us to 184.097 us.
Complete-call throughput regressed from 1.536020 to 1.451453 GP/s (-5.5%).
Camera-cholla regressed from 3.264633 to 3.241894 GP/s (-0.7%), and Calotes 4K
regressed from 3.704638 to 3.570457 GP/s (-3.6%).

## Decision

Reject and revert. The slow frame did not benefit because its expensive
emission work is not in the targeted Rice path; its 4.44x ratio and unchanged
kernel time instead motivate profiling and parallelizing fixed-block packing.

## References

- [EXP-0143](EXP-0143-cuda-warp-rice-emission.md)
- [CUDA feedback summary](../benchmarks/v5-cuda-feedback-encoder.md)
