# EXP-0054 — 8-bit tile-local rANS format

Status: **REJECTED**

## Classification

**Exploitation of the accepted EXP-0053 entropy frontier** — turn the
complete-byte model into an exact safe-Rust bitstream and measure the model
error and speed cost.

## Hypothesis

A version-3 8-bit stream with an independently decodable byte-rANS residual
mode will realize at least 10% complete-stream savings over the practical v2
frontier while preserving every reconstruction, tile boundary, GOP
dependency, and random-access result.

## Modification

- Reserve 8-bit entropy mode 10 for tile-local order-0 byte-rANS.
- Add 8-bit stream version 3; the decoder retains v0 and v2 compatibility,
  and the new entropy mode is rejected in older versions.
- For each rANS tile payload store:
  - table log in `[8, 12]`;
  - varint distinct-symbol count;
  - sorted folded-symbol deltas;
  - all but the final normalized frequency, with the final count implied by
    `2^table_log`;
  - a four-byte final rANS state;
  - reverse-order byte-renormalization data.
- Use a 32-bit byte-rANS state with lower normalization bound `2^23`.
- Normalize observed counts with the exact EXP-0053 largest-remainder rule.
- Compute exact output bytes for table logs 8 through 12 and choose rANS only
  when it is strictly smaller than the existing Rice/zero-run payload.
- Keep predictors, quantization, directory entries, and high-bit v2 streams
  unchanged.

## Correctness tests

- Exhaustive round trips for every 8-bit folded symbol and table-log
  boundary.
- Singleton, uniform, skewed, sparse, maximum-symbol, and long-tile streams.
- Encoder byte count matches the materialized payload exactly.
- Truncated state/data, invalid table log, duplicate or out-of-range symbols,
  zero/overflowing counts, noncanonical varints, invalid final state, and
  trailing bytes are rejected.
- Every predictor mode remains exact at q100 and respects the lossy error
  bound.
- v0/v2 reject the new entropy mode; v3 continues to decode legacy entropy
  modes.
- Individual-tile decode equals full-frame decode.

## Measurement

1. Run exact microtests and the four-case fast-feedback A/B against the
   practical v2 binary.
2. Compare actual rANS bytes with EXP-0053's normalized-frequency payload
   model.
3. If the size gate passes, run the complete 8-bit matrix and single-frame
   access confirmation.
4. Profile encode/decode before optimizing the entropy kernel.

## Gate

- Complete 8-bit bytes improve at least 10%.
- Actual bytes stay within 1% of EXP-0053's charged oracle.
- q100 is exact and every lossy reconstruction is byte-identical to v2.
- Initial encode/decode slowdowns are reported; a slowdown above 4x rejects
  the implementation structure, while a smaller slowdown may occupy a
  maximum-compression frontier branch pending exploitation.

## References

- [Research 0024: finite-block ANS entropy
  models](../research/0024-finite-block-ans-entropy-models.md)
- [EXP-0053: finite-block order-0 entropy
  model](EXP-0053-finite-block-order0-model.md)

## Result

The exact safe-Rust byte-rANS mode passed 50 release tests, strict Clippy, and
formatting. It round-tripped sparse, skewed, singleton, 511-symbol, and
32,768-sample payloads and rejected malformed tables, states, truncation, and
trailing data.

The six-trial fast-feedback A/B against the practical v2 binary measured:

| Case | Complete bytes | Encode throughput | Decode throughput |
|---|---:|---:|---:|
| Camera 1080p | -5.24% | -82.46% | -27.85% |
| Scene cuts 1080p | -14.31% | -84.12% | -20.33% |
| Grid 4K | -44.14% | -85.63% | -34.94% |
| UI temporal 720p | -37.29% | -84.61% | -28.35% |
| Geometric mean | — | -84.25% | -28.05% |

The space result closely follows EXP-0053 and strongly passes the 10% intent
on three of four cases. The 0.1575x encode-throughput ratio is a 6.35x
slowdown, however, exceeding the predeclared 4x structural rejection gate.
The implementation scans every predictor residual sequence once for each of
five table logs and repeats the winning work during materialization.

Artifacts:

- `artifacts/exp0054-fast-feedback.tsv`
  (`b9c08ae92e9b23d9ae2dc7a676d2b9699ea1f6294316e518072d36997b4cf425`);
- `artifacts/frontier/fastvid-rans-exp0054`
  (`5905f54f4a7dcdb3f2bc21127ba634df704dd3e04b793745cc0b0b87e98d174c`).

## Decision

Reject the exhaustive table-log implementation structure while retaining the
version-3 syntax and exact coder as evidence. EXP-0055 exploits the accepted
EXP-0053 logarithmic model to select one table log and materializes rANS only
for the selected predictor.

