# EXP-0109 — Speed-policy test reconciliation

Status: **ACCEPTED**

## Classification

**Evaluation integrity** — reconcile five stale policy assertions with the
promoted speed-tier encoder while preserving behavioral coverage.

## Hypothesis

The five full-suite failures retained since EXP-0099 are tests of retired
encoder-selection policy, not codec correctness regressions. Replacing those
assumptions with direct invariant checks should make the full release suite
green without changing production codec behavior:

- legacy decoders still accept only legacy modes;
- predictor oracle results are still exact minima with bounded error;
- scalar/four-state rANS kernels retain direct round-trip validation; and
- independently decoded tiles still equal full-frame decode.

## Modification

Tests only; no production encoder, decoder, stream, selector, or metric
changes.

## Gate

- all five formerly failing tests pass for the intended invariant;
- the complete release suite passes;
- strict Clippy, formatting, and diff checks pass;
- production-code diff is empty.

## Result

The five assertions were coupled to encoder-selection policies retired by
the speed-tier work, while the underlying codec invariants remained covered
by direct kernel and round-trip tests. The replacements now check:

- the predictor oracle is no larger than every candidate and every candidate
  obeys the quantizer error bound (with zero squared error at q100);
- the entropy mode selected by the current encoder is a supported mode and
  independent tile decode equals full decode exactly; and
- a legacy-compatible Paeth stream still decodes, while legacy versions
  continue to reject newer predictor modes.

`cargo test --release` passes all 65 library tests, both motion-model tests,
both squeeze-model tests, all binary targets, and documentation tests. Strict
release Clippy, formatting, and `git diff --check` also pass.

The diff is confined to `#[cfg(test)]` test code and one test-only helper.
There is no production encoder, decoder, format, selector, or metric change,
and therefore no performance benchmark is warranted.

## Decision

Accept the test reconciliation. The full suite is again a usable correctness
gate for subsequent codec experiments.

## References

- [EXP-0099](EXP-0099-interleaved-rice-tile-pairs.md)
- [EXP-0108](EXP-0108-bounded-shard-stream-prototype.md)
