# EXP-0107 — Combined wavefront/entropy execution model

Status: **ACCEPTED**

## Classification

**Parallel-format exploitation** — combine accepted predictor and entropy
branches with complete control-byte accounting.

## Hypothesis

Sixty-four-row independent clamp-gradient bands followed by 4,096-symbol
raster entropy shards should bound predictor storage and every entropy
state while retaining acceptable rate on the native high-bit q90 corpus:

- maximum predictor work unit at most 16,384 samples;
- maximum entropy span at most 4,096 symbols;
- aggregate complete bytes no more than 3% above unsplit clamp-gradient;
- no sample more than 5% above unsplit clamp-gradient; and
- squared error and maximum error identical to EXP-0104's 64-row model.

## Modification

Extend the read-only 64-row predictor-band model. Inside each band:

- retain raster residual order, following Research 0038 and EXP-0106;
- split folded residuals into 4,096-symbol shards;
- independently choose exact zero-run or four-lane Rice for every shard;
- for Rice, round-robin symbols over four byte-aligned lanes;
- charge three `u32` lane lengths, a one-byte mode/parameter per shard, one
  `u32` length for every non-final shard, and the existing five-byte
  predictor-band boundary.

No encoder, decoder, or bitstream changes.

## Gate

- all stated span/rate/error conditions pass;
- targeted exact accounting tests, formatting, and strict Clippy pass;
- results are reported per sample/depth and checksummed.

## Result

The native high-bit q90 run covers 4,752 tiles. Frozen EXP-0104 control
columns match exactly for every row, proving that the new entropy model did
not change 64-row predictor bytes or reconstruction error.

| Sample | Depth | Unsplit clamp bytes | Combined bytes | Delta |
|---|---:|---:|---:|---:|
| HDR gradient | 10 | 1,764,346 | 1,784,314 | +1.1318% |
| Precision motion | 10 | 18,812,972 | 19,030,698 | +1.1573% |
| Precision UI | 12 | 1,174,242 | 1,203,599 | +2.5001% |
| Precision motion | 16 | 4,724,937 | 4,915,079 | +4.0242% |
| **Aggregate** | mixed | **26,476,497** | **26,933,690** | **+1.7268%** |

Maximum predictor work unit is 16,384 samples and maximum entropy span is
4,096 symbols. Every span/rate gate passes. The combined cost is 0.970
percentage points above EXP-0104's 64-row-only aggregate delta (+0.7564%),
so the bounded entropy layout consumes less than one additional percentage
point of the unsplit payload.

Artifact:
`artifacts/exp0107-combined-execution.tsv`
(`aed45fd7af114a8af27a91357a3bf9728166626eed19550acbe50b354a95e1ec`).

The targeted exact zero-run/Rice lane accounting and predictor-boundary tests
pass. Formatting, strict release Clippy, Python compilation, shell syntax,
and diff checks pass. The full release suite retains the five pre-existing
speed-tier policy-test failures documented by EXP-0099; none exercises or is
changed by this read-only model.

## Decision

Accept the combined 64-row predictor/four-lane entropy point as the leading
bounded-serialization format candidate. It reduces the predictor span by 2x
and the entropy span by at least 8x relative to a full 32,768-symbol luma
tile for +1.727% aggregate modeled bytes, while preserving raster locality
and EXP-0104's error exactly.

Do not promote it into an active codec slot yet. The next implementation step
needs normative syntax and a scalar round-trip prototype, followed by
byte-exact tests and CPU timing. CUDA performance remains unmeasured.

## References

- [Research 0038](../research/0038-lossless-wavefront-scheduling.md)
- [EXP-0102](EXP-0102-four-lane-rice-shard-model.md)
- [EXP-0104](EXP-0104-predictor-band-height-ladder.md)
- [EXP-0105](EXP-0105-predictor-wavefront-model.md)
- [EXP-0106](EXP-0106-diagonal-residual-order-model.md)
