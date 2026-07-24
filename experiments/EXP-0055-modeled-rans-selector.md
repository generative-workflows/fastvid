# EXP-0055 — Modeled rANS selector

Status: **PENDING**

## Classification

**Exploitation of the EXP-0054 entropy candidate** — remove redundant exact
rANS simulations while preserving the version-3 format and nearly all modeled
space savings.

## Hypothesis

Choosing the table log from the EXP-0053 normalized logarithmic cost and
materializing exact rANS only for the selected predictor will improve encode
throughput at least 2x relative to EXP-0054 while keeping complete bytes
within 1% of the exhaustive candidate.

## Modification

- Normalize and score table logs 8 through 12 from histograms only.
- During predictor selection, use the modeled complete rANS bytes rather than
  simulating state renormalization for every table log and predictor.
- Build one exact rANS payload for the selected predictor and retain it only
  when its materialized size is strictly smaller than Rice/zero-run.
- Keep decoder, table syntax, normalization, reconstruction, and legacy
  compatibility unchanged.

## Test

- On synthetic histograms, compare modeled and exact sizes for every table
  log and record the selected-log mismatch rate.
- Require byte-identical reconstruction and exact q100 output.
- Run the six-trial fast-feedback A/B against both practical v2 and preserved
  EXP-0054.

## Gate

- At least 2x encode-throughput improvement over EXP-0054.
- Complete bytes no more than 1% larger than EXP-0054 in any fast case.
- Decode throughput unchanged within 5% relative to EXP-0054.
- If promising, advance to the complete 8-bit matrix and profile remaining
  encoder/decoder costs.

## References

- [EXP-0053](EXP-0053-finite-block-order0-model.md)
- [EXP-0054](EXP-0054-8bit-tile-rans-format.md)

