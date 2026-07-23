# EXP-0012 — Dynamic scheduling with private tile results

Status: **REJECTED**

## Hypothesis

Keeping the atomic dynamic work queue while accumulating `(index, value)` pairs
privately per worker will remove per-tile result-lock contention without the
load imbalance observed in [EXP-0011](EXP-0011-parallel-map-contention.md).

## Modification

Each scoped worker retains the atomic index queue but writes completed tile
results into a private vector. After workers join, the calling thread places
the pairs into the ordered result vector. No mutex is needed in the worker hot
path.

## Test

1. Compare two five-trial fast-tier candidate runs with the accepted EXP-0010
   baseline.
2. Require bit-identical sizes and all unit tests.
3. If the fast tier passes, run the standard q90/q100, one/four-thread, GOP-1
   corpus confirmation.

## Acceptance criteria

- Four-thread temporal encode or decode improves by at least 3% without a
  greater than 2% regression in the other direction.
- One-thread cases do not regress by more than 2%.
- Full-corpus confirmation agrees with the fast-tier direction.

## Results

Two candidate runs compared with the two accepted EXP-0010 baselines:

| Run | UI encode | UI decode | Aggregate encode | Aggregate decode |
|---|---:|---:|---:|---:|
| Baseline A | 140.637 | 204.035 | 58.318 | 86.456 |
| Baseline B | 141.210 | 204.297 | 56.334 | 86.736 |
| Candidate A | 139.964 | 203.395 | 56.819 | 86.284 |
| Candidate B | 144.734 | 207.278 | 58.597 | 87.538 |

All throughput values are MP/s. Candidate B's UI result improved 2.50% encode
and 1.46% decode over baseline B, but candidate A regressed 0.48% and 0.31%.
The effect is within run-to-run noise and did not reach the 3% acceptance
threshold. Tests passed and every encoded size was unchanged.

## Conclusion

Rejected. A per-tile uncontended mutex is not a material bottleneck at the
current tile size, and private completion vectors add their own allocation and
merge work. The original dynamic implementation is restored.


## References

- [Research 0012](../research/0012-simd-cache-profiling.md)
- [EXP-0010](EXP-0010-fast-feedback-loop.md)
- [EXP-0011](EXP-0011-parallel-map-contention.md)
