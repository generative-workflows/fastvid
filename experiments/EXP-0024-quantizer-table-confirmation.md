# EXP-0024 — Quantizer-table corpus confirmation

Status: **ACCEPTED**

## Hypothesis

The frame-local quantizer table provides a robust corpus-wide encode
improvement by eliminating per-sample division, while unchanged decode stays
within the validated ±3% A/B noise band.

## Modification

Retest the unchanged EXP-0023 implementation using preserved post-EXP-0021
baseline and quantizer-candidate binaries.

## Test

Run the 12-still q90/q100 one/four-thread matrix with six balanced alternating
trials per binary. Verify the lookup against scalar quantization exhaustively.

## Acceptance criteria

- Every encode geomean improves by at least 8%.
- No individual encode median regresses.
- All decode geomeans remain within 3%.
- Encoded bytes and quality metrics are identical.

## Results

Balanced six-trial changes across all 12 stills:

| Quality | Threads | Encode geomean | Decode geomean | Worst encode row |
|---:|---:|---:|---:|---:|
| 90 | 1 | **+22.60%** | +0.19% | +11.11% |
| 90 | 4 | **+20.45%** | -1.89% | +16.63% |
| 100 | 1 | **+16.47%** | -0.45% | +4.48% |
| 100 | 4 | **+14.03%** | +0.28% | +6.76% |

Every encode row improved. All encoded bytes and quality metrics matched.
Exhaustive unit testing covered all 511 residuals at every legal quantization
step. Native assembly shows the inner temporal loop loading signed table
entries with `movswl`; the only remaining `idivl` instructions in this path
construct the 511-entry table once per frame.

## Conclusion

Accepted. A 1,022-byte immutable table removes per-sample division and yields
a large safe-Rust improvement without changing the format.


## References

- [EXP-0020](EXP-0020-balanced-ab-harness.md)
- [EXP-0021](EXP-0021-entropy-allocation-final.md)
- [EXP-0023](EXP-0023-quantizer-lookup-table.md)
