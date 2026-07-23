# EXP-0013 — Incremental decode coordinates

Status: **REJECTED**

## Hypothesis

Tracking tile x/y coordinates incrementally in entropy decode loops will avoid
per-sample division and remainder operations, improving decode throughput
without affecting compression or reconstruction.

## Modification

Pass the current tile-local x/y coordinates into sample reconstruction.
Advance them once per decoded sample and wrap x at tile width. Remove
`index % width` and `index / width` from the reconstruction hot path.

This is a safe scalar-loop restructuring intended both to reduce instruction
count now and to make temporal reconstruction more amenable to later
auto-vectorization.

## Test

1. Compare two five-trial fast-tier runs with EXP-0010.
2. Inspect optimized assembly to confirm the per-sample division is absent.
3. Run unit tests and require byte-identical sizes.
4. If accepted, confirm on the standard q90/q100, one/four-thread, GOP-1
   corpus matrix.

## Acceptance criteria

- Decode geomean improves by at least 3% in both candidate runs.
- No encode regression greater than 2%.
- Exact reconstruction and encoded sizes are unchanged.
- Full-corpus confirmation agrees with the fast-tier direction.

## Results

| Run | Baseline decode | Candidate decode | Change |
|---|---:|---:|---:|
| A | 86.456 MP/s | 89.093 MP/s | +3.05% |
| B | 86.736 MP/s | 88.680 MP/s | +2.24% |

The two-run mean improved 2.64%. Encode geomeans were 57.064 and
57.004 MP/s, within the 56.334–58.318 MP/s baseline range. Tests passed and
encoded sizes were unchanged.

## Conclusion

Rejected under the predeclared criterion because both decode runs did not
improve by at least 3%. The result suggests coordinate arithmetic is worth
revisiting as part of a larger specialized decoder, but the isolated change is
not strong enough to retain.


## References

- [Research 0012](../research/0012-simd-cache-profiling.md)
- [EXP-0010](EXP-0010-fast-feedback-loop.md)
