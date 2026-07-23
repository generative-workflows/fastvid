# EXP-0009 — Single-frame random-access benchmark

Status: **ACCEPTED**

## Hypothesis

A codec-only access benchmark will quantify the editing cost hidden by
sequential MP/s: GOP-12 worst-case target latency should require at most 12
decoded frames, while all-intra should require exactly one. The harness must
make the compression/access tradeoff explicit without changing reconstruction.

This follows [research 0010](../research/0010-single-frame-random-access.md)
and the accepted temporal baseline in
[EXP-0005](EXP-0005-gated-temporal-prediction.md).

## Modification

- Add a raw-sequence command that encodes frames outside the timed region,
  then decodes an isolated target from its nearest preceding keyframe.
- Record keyframe index, preroll/dependency count, decoded frames, compressed
  bytes read, access latency, useful-target throughput, actual-work
  throughput, and access amplification.
- Add a corpus harness covering offsets 0, 1, GOP/2, and GOP-1 in every GOP,
  with warm-up and five recorded trials.
- Define separate all-intra and GOP-12 summaries in the evaluation
  methodology.

## Test

On every corpus-v2 video at qualities 90 and 100 with one and four threads,
compare GOP 1 and GOP 12. Require decoded targets to equal the normal
reconstruction. Verify boundary targets, second-GOP targets, invalid target
indices, and nonzero GOP validation. Report median, p95, and worst access
latency and the corresponding byte/frame amplification.

## Results

Host: 4-vCPU AMD EPYC-Genoa VM, 7.6 GiB RAM, Rust 1.97.1. The standard matrix
contains 1,920 measured rows: six videos, eight identical target positions,
two qualities, two thread counts, two GOP modes, and five trials after
warm-up. Every requested target equals the normal sequential reconstruction.
The summarizer takes the median of five trials per target before computing
the 48-target corpus distribution.

| Quality | Threads | GOP | Median access | p95 access | Worst access | Mean amplification |
|---:|---:|---:|---:|---:|---:|---:|
| 90 | 1 | 1 | 34.65 ms | 39.20 ms | 39.70 ms | 1.0x |
| 90 | 1 | 12 | 88.33 ms | 409.69 ms | 434.13 ms | 5.5x |
| 90 | 4 | 1 | 9.54 ms | 10.89 ms | 11.15 ms | 1.0x |
| 90 | 4 | 12 | 23.15 ms | 113.76 ms | 115.47 ms | 5.5x |
| 100 | 1 | 1 | 36.40 ms | 39.77 ms | 40.79 ms | 1.0x |
| 100 | 1 | 12 | 89.80 ms | 419.48 ms | 472.18 ms | 5.5x |
| 100 | 4 | 1 | 9.98 ms | 11.40 ms | 11.69 ms | 1.0x |
| 100 | 4 | 12 | 24.58 ms | 121.85 ms | 122.82 ms | 5.5x |

The GOP-12 median is 2.5x the all-intra median at one thread, but p95 is over
10x and worst access 10.9–11.6x because targets late in the GOP require twelve
decodes. Four threads reduce absolute latency substantially but cannot remove
the dependency and byte amplification. Worst one-thread GOP-12 results occur
on `ed-dense-motion`; the quality-100 maximum is 472.18 ms at target 11.

All fourteen Rust tests, strict Clippy, release builds, malformed target/GOP
validation, and target-equivalence checks pass. Container/index and cold-cache
I/O remain explicitly outside this codec-only baseline.

## Decision

Accept. The harness exposes a material editing tradeoff that sequential MP/s
conceals and establishes all-intra as the direct-access baseline. GOP-12
retains its compression advantage, but future temporal improvements must
report isolated-frame p95/worst latency and dependency amplification. A
sequence container and keyframe index are the next prerequisites for measuring
end-to-end seek cost.
