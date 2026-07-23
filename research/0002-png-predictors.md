# 0002 — PNG spatial predictors

Source: W3C, “Portable Network Graphics (PNG) Specification, Third Edition,”
filtering sections.

- Specification: https://www.w3.org/TR/png-3/#9Filtering
- Terms: W3C permissive document/code licensing applies; the predictor is also
  described here as literature rather than copied implementation.

## Findings

PNG defines causal byte predictors using left, above, and upper-left samples.
The Paeth predictor chooses the neighbor closest to `left + above -
upper_left`. It is cheap, reversible when residuals are exact, and needs only
the current and previous reconstructed rows.

## Fastvid implications

Use Paeth as the first spatial baseline. Reset its context at every tile edge
to preserve tile independence. In lossy modes, prediction must use
reconstructed—not original—neighbors, or encoder and decoder states drift.

## Relevant experiments

- [EXP-0001](../experiments/EXP-0001-paeth-varint-baseline.md) implements and
  measures the tile-local Paeth baseline.
