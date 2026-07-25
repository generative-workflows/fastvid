# EXP-0098 — Partial Rice-4 fallback

Status: **REJECTED**

## Classification

**Profile-directed exploitation** — preserve useful packed work when a
four-symbol Rice group crosses the 64-bit temporary-word boundary.

## Hypothesis

EXP-0097 attributes 28.80% of encode-only samples to scalar Rice writing.
The accepted group kernel currently discards a packable prefix and sends all
four values through the scalar writer when any later value overflows the
temporary word. Emitting the prefix once and falling back only from the
overflowing value should improve matched q90 one-thread encode by at least
2% without changing a byte.

## Modification

1. Factor the existing at-most-64-bit writer-state append into a private
   helper.
2. On group overflow, append the packed prefix and scalar-write only the
   current and remaining values.
3. Keep the existing all-packed path, Rice-0/Rice-4 dispatch, block packer,
   selector, syntax, and decoder unchanged.

## Gate

- exact specialized/scalar writer equivalence across existing boundary
  groups and alignments;
- byte- and metric-identical focused q90/q100 streams;
- at least 2% matched q90 one-thread encode improvement over EXP-0095;
- no focused encode cell regresses more than 3%;
- decode no worse than 5%;
- strict Clippy, formatting, and relevant tests pass; and
- no slow tier unless the focused gate passes.

## Result

Strict release Clippy, formatting, and the specialized/scalar writer
equivalence test passed. The focused candidate remained byte- and
metric-identical.

A balanced two-trial q90 one-thread screen measured:

| Depth | Baseline encode | Candidate encode | Delta | Decode delta |
|---:|---:|---:|---:|---:|
| 10-bit | 72.303 MP/s | 71.286 MP/s | -1.407% | +0.461% |
| 16-bit | 65.938 MP/s | 64.618 MP/s | -2.001% | -1.086% |
| geometric aggregate | 69.047 MP/s | 67.870 MP/s | -1.704% | -0.315% |

The primary path regressed instead of meeting the +2% gate. No six-trial or
slow-tier run was performed.

Artifacts:

- focused raw results:
  `artifacts/exp0098-partial-rice4-smoke.tsv`
  (`182e0d3268c49474715aec03e46817d05c27c9a5600342d9e87dc35ef262dc72`);
- candidate binary:
  `artifacts/frontier/fastvid-speed-exp0098-partial-rice4`
  (`ab250696a8170c373baca926dd1f1f23563ae3b716177cb1715c7af86b748131`).

## Decision

Reject partial fallback and restore EXP-0095 exactly. The extra writer-state
append and fallback split cost more than the avoided scalar calls on both
tested depths. Combined with EXP-0080, EXP-0083/84, EXP-0092, and this
result, local Rice control-flow reshaping is sufficiently explored.

Move the next speed experiment to a predictor-loop layout or a different
tech-tree branch rather than another group-fallback variant.

## References

- [EXP-0084](EXP-0084-specialized-rice-batching.md)
- [EXP-0095](EXP-0095-block-pack-rice4-combination.md)
- [EXP-0097](EXP-0097-post-rice4-profile.md)
