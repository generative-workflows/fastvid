# EXP-0017 — Deferred zero-run payload materialization

Status: **REJECTED**

## Hypothesis

Counting the prospective zero-run payload length while accumulating residuals,
then materializing bytes only when zero-run coding wins, will reduce allocation
and write traffic for dense Rice-coded tiles enough to improve aggregate encode
throughput.

## Modification

- Replace the eagerly written zero-run payload with an exact byte-count
  accumulator.
- Preserve folded residuals and the Rice histogram.
- If Rice wins, never allocate or write zero-run bytes.
- If zero-run wins, make a second sequential pass over the folded residual
  vector and emit the canonical payload with exact capacity.

This preserves the bitstream byte-for-byte.

## Test

1. Run two five-trial fast-tier candidate matrices.
2. Compare against the two post-EXP-0015 baselines.
3. Require exact encoded sizes and reconstruction.
4. If accepted, run focused corpus confirmation at q90/q100 and one/four
   threads.

## Acceptance criteria

- Camera/Rice encode improves by at least 5% in both runs.
- Aggregate encode geomean improves by at least 3% in both runs.
- No zero-run case regresses encode by more than 3%.
- Decode stays within 2%, and encoded bytes/quality remain identical.

## Results

| Case | Baseline range | Candidate A | Candidate B | Change range |
|---|---:|---:|---:|---:|
| grid-4k | 37.768–38.099 | 45.923 | 45.537 | +19.5% to +21.6% |
| camera-1080p | 22.500–22.637 | 23.576 | 23.859 | +4.8% to +6.0% |
| ui-temporal-720p | 133.991–138.388 | 159.551 | 167.836 | +15.3% to +25.3% |
| cuts-temporal-1080p | 93.819–97.754 | 108.379 | 106.822 | +9.3% to +15.5% |
| aggregate | 57.295–58.317 | 65.779 | 66.434 | +12.8% to +16.0% |

All throughput values are encode MP/s. Decode stayed within normal run
variation, and encoded bytes were identical. Camera candidate A improved only
4.78% relative to baseline A, narrowly failing the requirement that both runs
improve the Rice case by at least 5%.

The unexpectedly broad improvement points to allocation capacity rather than
only discarded writes: the old zero-run vector reserved one byte per sample
for every tile even when the eventual payload was much smaller.

## Conclusion

Rejected under its Rice-specific gate. The broader allocation hypothesis and
substantial aggregate result are retested across every corpus still in
[EXP-0018](EXP-0018-exact-entropy-allocation.md).


## References

- [Research 0012](../research/0012-simd-cache-profiling.md)
- [EXP-0010](EXP-0010-fast-feedback-loop.md)
- [EXP-0016](EXP-0016-entropy-mode-instrumentation.md)
