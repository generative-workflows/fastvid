# EXP-0036 — Fused zero-run and first Rice analysis

Status: **REJECTED**

## Hypothesis

Computing zero-run encoded length and the Rice-parameter-zero quotient sum in
one folded-residual traversal will remove one cache-active pass from
`finish_entropy` and improve high-bit encoding without changing its parameter
selection or payload.

## Modification

Replace the independent zero-run sizing pass and parameter-zero Rice sum with
one analysis loop. Continue parameters 1–16 in the existing parameter-outer
form, preserving the compiler-friendly loop structure retained after
EXP-0028 and the exact early termination from EXP-0029.

No residual-generation, stream-format, or decoder changes.

## Test

1. Preserve the accepted EXP-0032 binary.
2. Extend the exhaustive Rice selector oracle to compare the fused analyzer's
   parameter and bit count with a full independent scan.
3. Require byte-identical streams on the balanced high-bit video fast matrix.
4. Advance only if encode geomean improves by at least 3% with no cell
   regressing more than 3%.
5. Confirm an accepted candidate on the full supplement and repeat EXP-0034
   PMU counters.

## Acceptance criteria

- Rice choice, predicted bit count, stream bytes, and quality are unchanged.
- Fast and confirmation encode geomeans improve by at least 3%.
- Decode remains within the 5% noise gate.
- PMU evidence is directionally consistent with one fewer residual pass.

## Results

Release tests, the exhaustive full-scan Rice oracle, formatting, and strict
Clippy passed. All fast-matrix encoded bytes and quality signatures were
identical.

The four-trial balanced video result was:

- encode geomean: **+0.79%**;
- decode geomean: -0.70%;
- encode cell range: -4.31% to +4.08%.

The candidate did not clear the 3% advancement threshold and the 16-bit
q100/four-thread cell exceeded the -3% regression gate. The removed traversal
was therefore not worth the more complex combined branch/sum loop.

Artifact: `artifacts/exp0036-fast-gop1.tsv`.

## Conclusion

Rejected and reverted at the fast gate. `finish_entropy` remains the correct
profiled region, but combining zero-run accounting with the first Rice scan
does not produce a reliable wall-time benefit. PMU confirmation was skipped
because the primary performance gate failed.

## References

- [EXP-0028](EXP-0028-single-pass-high-bit-rice-cost.md)
- [EXP-0029](EXP-0029-rice-cost-early-termination.md)
- [EXP-0034](EXP-0034-perf-samply-cache-profile.md)
