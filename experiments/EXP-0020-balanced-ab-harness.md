# EXP-0020 — Balanced alternating A/B harness

Status: **ACCEPTED**

## Hypothesis

Using six alternating trials will give each binary three first and three second
positions per cell, reducing order bias enough for the unchanged decode path to
serve as a valid negative control.

## Modification

Require an even trial count in `benchmark-ab-corpus.sh` and use six trials for
the EXP-0018 decision.

## Test

Repeat the complete 12-still q90/q100 one/four-thread matrix, reduce to
per-variant medians, and verify trial balance and invariants.

## Acceptance criteria

- Equal baseline/candidate and first/second execution counts.
- Encoded bytes and quality are identical.
- All unchanged decode geomeans remain within 3%.
- Results are sufficiently stable to decide EXP-0018.

## Results

Six trials gave every binary three first and three second positions. All
encoded bytes and quality metrics matched. Unchanged decode geomean differences
were +1.28%, -2.55%, -0.36%, and +2.46%, all within 3%.

## Conclusion

Accepted. Even trial counts are mandatory for this A/B harness.


## References

- [EXP-0018](EXP-0018-exact-entropy-allocation.md)
- [EXP-0019](EXP-0019-alternating-ab-harness.md)
