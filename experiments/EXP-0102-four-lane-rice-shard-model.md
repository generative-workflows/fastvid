# EXP-0102 — Four-lane Rice shard model

Status: **ACCEPTED**

## Classification

**Entropy-format exploration** — complete-byte model of independently
locatable Rice lanes inside 4,096-symbol execution shards.

## Hypothesis

Splitting every currently selected Rice tile into 4,096-symbol shards and
round-robin assigning each shard to four byte-aligned Rice lanes will reduce
the maximum entropy-state span to 1,024 symbols. Charging one `u32` length for
every lane except the last and every shard except the last should increase
aggregate selected-Rice bytes by less than 2% and complete tile payload bytes
by less than 1% on the native high-bit q90 supplement.

## Modification

Extend the read-only entropy model with a four-lane candidate:

- retain the tile's selected Rice parameter and exact folded symbols;
- split at 4,096 symbols without resetting prediction;
- assign symbol `i` within a shard to lane `i mod 4`;
- byte-align every lane;
- charge three four-byte lane lengths per full shard and one four-byte shard
  length except for the final shard in a tile.

No encoder, decoder, selector, or bitstream behavior changes.
`scripts/benchmark-rice-lane-model.sh` runs the checksummed high-bit
supplement at q90/GOP1.

## Gate

- model output passes strict Clippy and targeted release tests;
- maximum modeled entropy span is 1,024 symbols;
- selected-Rice complete bytes regress by less than 2%;
- complete tile payload bytes regress by less than 1%;
- results are reported per bit depth/sample, with all controls charged.

## Result

Strict release Clippy, formatting, and the targeted exact accounting test
pass. The model covers 4,752 tiles, including 1,656 currently selected Rice
tiles:

| Sample | Depth | Rice tiles | Rice-byte delta | All-payload delta |
|---|---:|---:|---:|---:|
| HDR gradient | 10 | 72 | +1.3962% | +0.5083% |
| Precision motion | 10 | 720 | +1.3875% | +0.5055% |
| Precision UI | 12 | 144 | +1.6455% | +1.1015% |
| Precision motion | 16 | 720 | +1.8197% | +0.9708% |
| **Aggregate** | mixed | **1,656** | **+1.5098%** | **+0.6174%** |

Aggregate selected-Rice bytes increase from 10,609,321 to 10,769,503.
Complete tile payload bytes increase from 25,944,384 to 26,104,566. Both
predeclared aggregate gates pass. Each 4,096-symbol shard has four independent
round-robin states, so a full lane contains 1,024 symbols.

The result artifact is
`artifacts/exp0102-four-lane-rice.tsv`
(`41a2bf882e1e1bc97ae7ae003b9d8743a5450892eb6e7af99c212ffd8d993fe5`).

## Decision

Accept explicit four-lane Rice as a viable format branch, not as an
implemented codec change. It buys a 32x entropy-span reduction relative to a
full 32,768-symbol luma-tile stream for 0.617% aggregate payload cost.

Do not implement it yet. The spatial predictor still has a tile-wide causal
chain, so an entropy-only CUDA kernel would hand a parallel decoder residuals
that reconstruction cannot consume independently. The next format
exploration should compare independent predictor row bands/blocks, including
boundary rate cost, against a wavefront traversal. If that passes, this lane
layout becomes the entropy candidate for the same execution shards.

## References

- [Research 0037](../research/0037-parallel-hardware-friendly-codecs.md)
- [EXP-0100](EXP-0100-parallel-serialization-budget.md)
