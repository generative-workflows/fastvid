# EXP-0053 — Finite-block order-0 entropy model

Status: **ACCEPTED**

## Classification

**Exploration** — a table-driven entropy family distinct from Rice,
zero-run coding, predictor selection, and residual mappings.

## Hypothesis

The selected version-2 predictor residuals retain enough non-geometric
order-0 structure that a normalized-frequency coder can reduce complete
payload bytes by at least 2% after charging a tile-local sparse frequency
table and final state.

## Modification

Extend the read-only entropy analyzer. For every decoded folded-residual
sequence, compute:

- distinct symbol count;
- empirical order-0 entropy bytes with no model overhead;
- deterministic normalized-frequency payload bytes for table logs 8 through
  12, choosing the smallest complete result;
- sparse table bytes: one table-log byte, varint symbol count, sorted symbol
  deltas, all but one normalized counts, and four final-state bytes;
- per-tile oracle bytes between the current payload and the modeled order-0
  mode.

Normalization assigns one slot to each observed symbol, distributes remaining
power-of-two slots proportionally by largest remainder, and rejects table
sizes smaller than the alphabet. This is a size model only; it does not add an
ANS implementation or change the bitstream.

## Correctness tests

- Empty, singleton, all-zero, uniform, skewed, and maximum-alphabet
  histograms have independently calculated sizes.
- Normalized frequencies are positive for observed symbols and sum exactly to
  `2^table_log`.
- Modeled payload bits equal the sum of
  `count * log2(table_size / normalized_count)`, rounded once per tile.
- Sparse-table varint boundaries and implied final count are fully charged.
- Existing malformed-stream parsing and exact residual-count checks remain
  authoritative.

## Measurement

1. Screen the four fast-feedback cases at q90/q100.
2. If either the entropy lower bound or complete model passes its gate, run
   the full 8/10/12/16-bit matrix.
3. Report payload and complete-stream deltas, tile win rates, symbol-count
   percentiles, table/payload split, and groups by quality, content, bit
   depth, prediction, and current entropy mode.

## Gate

- Reject the entropy family if ideal order-0 bytes improve less than 3% in
  aggregate and no predeclared category improves 5%.
- Advance to a tile-local format prototype only if the completely charged
  per-tile oracle improves aggregate complete-stream bytes at least 2%, at
  least four content categories improve 1%, and no category expands.
- If ideal bytes improve at least 5% but the complete tile-local oracle misses
  2%, model frame/plane shared tables before rejecting ANS itself.

## References

- [Research 0024: finite-block ANS entropy
  models](../research/0024-finite-block-ans-entropy-models.md)
- [EXP-0038: byte-oriented residual format
  model](EXP-0038-byte-oriented-residual-model.md)

## Result

The complete matrix retained 164,664 tiles and 880 frames across all 18 core
samples at qualities 60/75/90/95/100 and the native 10/12/16-bit supplement
at q90/q100.

| Model | Complete bytes | Delta |
|---|---:|---:|
| Current practical v2 | 687,860,094 | — |
| Ideal empirical order-0 | 547,110,106 | -20.46% |
| Charged tile-local oracle | 553,214,283 | -19.58% |

The charged oracle saves 134,645,811 complete bytes. It selects the modeled
mode on 139,780 of 164,664 tiles (84.89%) and charges 5,400,245 bytes of
sparse tables on those winners. No tile was unsupported: distinct-symbol
counts were p50 6, p95 53, p99 115, and maximum 325.

Every predeclared category improved:

- AI-generated: -5.90%;
- camera: -7.70%;
- HDR gradient: -43.13%;
- high-precision motion: -55.90%;
- natural cinema: -18.89%;
- synthetic/UI: -11.28%.

The opportunity is present in both current entropy families: Rice-tile
payloads improve 15.09%, and zero-run-tile payloads improve 51.67%.
Spatial-prediction payloads improve 12.94% and temporal-prediction payloads
29.30%. Every bit depth and quality group improves.

Winning table logs were broadly distributed:

- log 8: 13,648 tiles;
- log 9: 13,039;
- log 10: 17,313;
- log 11: 22,934;
- log 12: 72,846.

Artifacts:

- `artifacts/exp0053-order0-screening.tsv`
  (`cf7414676dac978c4ede38224ac23b52aa484c7e7c3f621b46d3f1a7d82938dd`);
- `artifacts/exp0053-order0-model.tsv`
  (`e116d46308ec8d0c9b431f712a49ec1b05623d95d9f17ca4000c1f18a9fbd4b6`).

Analyzer binary:

- `target/release/entropy_model`
  (`46fa6087ae31fd2e5200fb6130e883d96d84c819c663bbac4710a98d8bed13e6`).

All 48 release library tests, every release target, strict Clippy, formatting,
and Lean passed.

## Decision

Accept the model and advance to a real tile-local entropy-format prototype.
The completely charged result exceeds the 2% gate by nearly an order of
magnitude, all six categories improve, and the small measured alphabets make
bounded tables practical. This acceptance proves a format opportunity, not
that the modeled logarithmic payload is already a conforming ANS stream; the
next experiment must implement exact safe-Rust encode/decode and compare
actual bytes with this model.
