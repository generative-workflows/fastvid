# EXP-0010 — Tiered fast-feedback benchmark loop

Status: **ACCEPTED**

## Hypothesis

A fixed, diverse four-case matrix can complete quickly enough for routine
optimization work while detecting regressions in spatial, temporal,
single-thread, and multi-thread paths. Full corpus runs can then be reserved
for confirmation.

## Modification

Add `scripts/benchmark-feedback.sh` and
`scripts/summarize-feedback.awk`. Define three evaluation tiers:

- fast: four pinned cases, five timed trials;
- medium: the standard corpus at one quality and selected thread/GOP settings;
- confirmation: the complete standard methodology.

The fast cases are:

1. 4K synthetic grid, q100, one thread, intra;
2. 1080p detailed camera still, q90, one thread, intra;
3. four 720p UI frames, q90, four threads, GOP 12;
4. four 1080p scene-cut/grain frames, q90, one thread, GOP 12.

## Test

1. Run the fast tier from a warm filesystem cache with five timed trials.
2. Record elapsed wall time and per-case median encode/decode throughput.
3. Re-run it and check that aggregate scores are stable enough to distinguish
   a five-percent change.
4. Use it for EXP-0011 iteration.
5. Run the complete corpus only after the fast tier accepts a candidate.

## Acceptance criteria

- A five-trial fast run completes in under 60 seconds on the reference
  four-vCPU host.
- Back-to-back aggregate encode and decode scores differ by less than 5%.
- The matrix exercises all four intended code-path categories.

## Results

The final matrix replaced the initially proposed 360p grid with a 4K grid. The
360p single-frame measurement was too short: unrelated runs moved from about
35 MP/s to 16 MP/s even though the one-thread code path was unchanged. The 4K
sample makes that path long enough to measure reliably.

Two back-to-back five-trial runs on the four-vCPU AMD EPYC-Genoa host produced:

| Run | Wall time excluding compilation | Encode geomean | Decode geomean |
|---|---:|---:|---:|
| A | 5.28 s inferred from the following no-build run | 58.318 MP/s | 86.456 MP/s |
| B | 5.39 s measured | 56.334 MP/s | 86.736 MP/s |
| Difference | — | 3.52% | 0.32% |

Run B per-case medians were:

| Case | Encode | Decode | Encoded bytes |
|---|---:|---:|---:|
| grid-4k | 37.649 MP/s | 58.109 MP/s | 2,383,708 |
| camera-1080p | 20.510 MP/s | 51.597 MP/s | 1,611,524 |
| ui-temporal-720p | 141.210 MP/s | 204.297 MP/s | 130,197 |
| cuts-temporal-1080p | 92.365 MP/s | 92.398 MP/s | 380,216 |

Encoded sizes were identical in both runs. Both aggregate differences were
below 5%, and the no-build loop completed in 5.39 seconds.

## Conclusion

Accepted. This is the default optimization feedback gate. Full-corpus runs are
confirmation evidence and must not be spent on every implementation attempt.

## References

- [Research 0011](../research/0011-openapv.md)
- [Research 0012](../research/0012-simd-cache-profiling.md)
- [Evaluation methodology](../EVALUATION_METHODOLOGY.md)
