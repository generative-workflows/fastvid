# EXP-0019 — Alternating baseline/candidate harness

Status: **REJECTED**

## Hypothesis

Alternating separately built baseline and candidate binaries within each
sample/trial cell will reduce time-correlated VM drift that made the initial
EXP-0018 sequential confirmation internally inconsistent.

## Modification

Add `scripts/benchmark-ab-corpus.sh`. It accepts explicit baseline and candidate
binaries, warms both, alternates execution order on odd/even trials, and writes
one interleaved TSV with a `variant` column.

## Test

Run the EXP-0018 still matrix with three trials and reduce each variant to
per-sample medians. Compare the unchanged decode path as a negative control.

## Acceptance criteria

- Every requested cell contains equal baseline/candidate trial counts.
- Encoded bytes and quality are identical between variants.
- Unchanged decode geomeans do not diverge by more than 3%.
- Results are usable to decide EXP-0018.

## Results

The harness produced equal row counts and invariant encoded bytes/quality, but
three trials gave unequal order: baseline ran first twice and candidate first
once. The unchanged decode geomean diverged +5.55% for q100 one-thread,
exceeding the 3% negative-control gate. Short 360p rows were especially noisy.

## Conclusion

Rejected. Alternation is necessary but trial order must also be balanced.
[EXP-0020](EXP-0020-balanced-ab-harness.md) requires an even trial count.


## References

- [Evaluation methodology](../EVALUATION_METHODOLOGY.md)
- [EXP-0018](EXP-0018-exact-entropy-allocation.md)
