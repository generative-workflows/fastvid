# EXP-0050 — High-bit streaming selector cost

Status: **REJECTED**

## Classification

**Exploitation of the EXP-0048 compression candidate** — reduce the exact
multi-predictor selector cost without changing its choices or stream.

## Hypothesis

Maintaining zero-run and Rice costs incrementally while residuals are
generated will eliminate four full residual-vector scans per tile and improve
high-bit encode throughput materially, while preserving exact EXP-0047 oracle
bytes and modes.

## Modification

- Add a high-bit residual accumulator that stores folded residuals for final
  emission while updating:
  - canonical zero-run varint byte cost;
  - Rice bit costs for every supported parameter.
- Select the entropy mode and predictor from cached exact costs.
- Materialize only the winning payload.
- Do not alter prediction, quantization, entropy syntax, tie breaking, or
  decoding.

## Correctness tests

- Exhaustively compare streaming costs with the existing entropy model at
  boundary residuals and representative vectors.
- Candidate bytes and predictor modes remain unchanged.
- All release tests, strict Clippy, formatting, and Lean pass.

## Measurement

1. Run the focused high-bit q90 video A/B loop.
2. Run the standard 8-bit fast-feedback loop as a control.
3. Confirm exact sizes against the EXP-0048 artifacts.

## Acceptance gate

- Encoded bytes do not change.
- High-bit focused encode throughput improves at least 10% relative to the
  EXP-0049 candidate.
- The preserved-baseline high-bit encode slowdown falls below 4x on the
  focused geometric mean.
- Decode throughput remains within measurement noise of EXP-0049.

## Result

The implementation preserved encoded sizes exactly but reduced focused
high-bit q90 encode throughput:

- EXP-0049 specialized candidate: `0.1974x` the balanced baseline geometric
  mean;
- streaming-cost candidate: `0.1361x` the balanced baseline geometric mean.

The 10-bit candidate/baseline ratio fell from `0.2179x` to `0.1682x`; the
16-bit ratio fell from `0.1789x` to `0.1101x`. Decode ratios were unchanged
within noise, including the expected unresolved 16-bit spatial-zero-run
regression.

Artifact:

- `artifacts/exp0050-highbit-q90-video-ab.tsv`
  (`dd0fa1dfd5994d23261aff793437be622e42b5da13f522e5c6af43fddf89a419`).

## Decision

Reject and revert. Maintaining 17 Rice quotient sums in the predictor's
innermost loop is substantially more expensive than scanning the compact,
cache-resident residual arrays after prediction. A future exact selector
should reduce the number of predictors reaching full entropy evaluation
rather than interleave all entropy arithmetic with reconstruction.
