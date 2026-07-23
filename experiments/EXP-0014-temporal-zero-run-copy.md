# EXP-0014 — Bulk-copy temporal zero runs

Status: **REJECTED**

## Hypothesis

For temporal tiles, a zero residual reconstructs exactly to the reference
sample. Copying contiguous row spans for zero-run tokens will replace
per-sample coordinate calculation, branching, and stores with optimized slice
copies, materially improving decode throughput on low-motion content.

## Modification

Specialize zero-run decoding when prediction mode is temporal:

- split each run at tile-row boundaries;
- copy the corresponding contiguous reference-plane span directly into the
  tile output;
- retain scalar reconstruction for nonzero residuals and spatial tiles.

`copy_from_slice` stays in safe Rust and allows the standard library/compiler
to select architecture-optimized memory-copy code without an unsafe SIMD
module.

## Test

1. Compare two five-trial fast-tier runs with EXP-0010.
2. Require exact reconstruction and unchanged encoded sizes.
3. If accepted, run the standard q90/q100 one/four-thread GOP-1 corpus matrix.

## Acceptance criteria

- The UI temporal decode median improves by at least 5% in both runs.
- Aggregate decode geomean improves by at least 3% in both runs.
- Encode remains within 2% of baseline.
- Full-corpus confirmation has no decode geomean regression and improves at
  least one temporal video by 5%.

## Results

| Run | UI decode | Scene-cut decode | Aggregate decode |
|---|---:|---:|---:|
| Baseline A | 204.035 | 88.414 | 86.456 |
| Baseline B | 204.297 | 92.398 | 86.736 |
| Candidate A | 206.938 | 205.200 | 107.051 |
| Candidate B | 208.039 | 200.768 | 106.399 |

All values are MP/s. UI improved only 1.42% and 1.83%, failing the required
5% in both runs. Aggregate decode nevertheless improved 23.8% and 22.7%
because the scene-cut/grain case improved 132.1% and 117.3%. Encode remained
within the baseline range, tests passed, and encoded sizes were unchanged.

## Conclusion

Rejected under its low-motion/UI-specific acceptance criteria. The unexpected
large win on another temporal content class warrants the broader corpus test in
[EXP-0015](EXP-0015-temporal-copy-corpus-confirmation.md).


## References

- [Research 0012](../research/0012-simd-cache-profiling.md)
- [EXP-0010](EXP-0010-fast-feedback-loop.md)
- [EXP-0013](EXP-0013-incremental-decode-coordinates.md)
