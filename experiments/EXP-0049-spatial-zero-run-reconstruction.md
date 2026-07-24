# EXP-0049 — Spatial zero-run reconstruction

Status: **REJECTED**

## Classification

**Exploitation of the EXP-0048 compression candidate** — recover decode
performance after tile-local predictor selection shifts long zero runs from
the previous-frame predictor to spatial predictors.

## Hypothesis

Reconstructing a spatial zero run by row spans, with the prediction mode
validated once per run, will reduce high-bit decode time without changing a
single encoded byte or decoded sample.

## Modification

- Keep the version-2 encoder and bitstream unchanged.
- For temporal zero runs, retain the existing row-span copy from the
  reference frame.
- For spatial zero runs, validate the predictor once and reconstruct each
  row span directly, avoiding the generic per-sample result path and repeated
  mode dispatch.
- Cache candidate payload lengths during encoder selection so entropy costs
  are not recomputed during tie breaking.

## Correctness tests

- Candidate encoded sizes and oracle mode choices remain unchanged.
- All release tests, strict Clippy, formatting, and Lean pass.
- q100 remains exact and lossy maximum-error bounds remain unchanged.

## Measurement

1. Run the short preserved-binary 8-bit A/B loop.
2. Run a focused six-trial high-bit q90 video A/B, including the 16-bit
   motion sequence that exposed the regression.
3. If the focused result is promising, confirm decode and single-frame
   access on the full EXP-0048 matrix.

## Acceptance gate

- Encoded bytes do not change.
- Focused high-bit decode geometric mean does not regress more than 10%.
- No individual decode cell regresses more than 15% without a documented
  mechanism and follow-up.
- Encode performance does not regress relative to the fused EXP-0048
  selector.

## Result

The first six-trial high-bit q90 video comparison used candidate binary
`312b8a5a9dedf6caf7bd839edb124fba02c261b1477abf881aa9dbc3852aee75`.
Row-span reconstruction improved the 16-bit decode regression from the
EXP-0048 measurement of about 67.7% to 60.3%, while 10-bit decode regressed
3.1%. Encoded bytes remained exactly unchanged.

Specializing each spatial mode outside the sample loop, including a direct
Paeth path, produced candidate binary
`6546e59753ae653394de1ad6088d610dcd09c66bc8438cdd9b7ea804ae6ee6ee`.
The four-trial confirmation measured:

- 10-bit motion q90: 26.92% fewer bytes, 78.21% lower encode throughput, and
  1.57% lower decode throughput;
- 16-bit motion q90: 28.12% fewer bytes, 82.11% lower encode throughput, and
  59.19% lower decode throughput;
- focused geometric mean: 80.26% lower encode throughput and 36.62% lower
  decode throughput.

Artifacts:

- `artifacts/exp0049-highbit-q90-video-ab.tsv`
  (`2687407a7fd39efd4bdd8ad8e127ab15636485657245a35efc344d0550f04d47`);
- `artifacts/exp0049-specialized-mode-q90-video-ab.tsv`
  (`4a262b8f48a52ea47d5d4e5befbfcd394e33be725eb8dc11d6733008118830b1`).

## Decision

Reject. The optimization preserves the space result and helps slightly, but
misses both decode gates by a wide margin. The remaining cost is the serial
spatial prediction dependency chain itself, not generic result handling or
mode dispatch. Retain the implementation as a harmless decoder improvement,
but do not claim it solves EXP-0048's decode regression.
