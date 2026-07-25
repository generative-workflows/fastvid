# EXP-0103 — Independent predictor bands

Status: **REJECTED**

## Classification

**Predictor-format exploration** — replace tile-wide reconstruction state
with independently reconstructed 16-row bands.

## Hypothesis

Resetting clamp-gradient state every 16 rows bounds a full luma execution
band to 4,096 samples while retaining reconstructed-neighbor prediction
inside the band. After charging one entropy-mode byte and one `u32` length
for every added band, aggregate complete bytes should regress by less than
3% against tile-wide clamp-gradient and aggregate squared error by less than
1% on the native high-bit q90 supplement.

## Modification

Extend the read-only predictor model:

- split each existing access tile into 16-row bands;
- independently initialize above/left state for every band;
- apply the existing clamp-gradient predictor and quantizer exactly;
- independently select the current zero-run/Rice entropy representation;
- retain the access tile's existing first mode/length entry and charge five
  bytes for every additional band;
- sum exact payload bytes, squared error, and maximum error.

No encoder, decoder, selector, or format changes.

## Gate

- strict Clippy, formatting, and exact model tests pass;
- no band contains more than 4,096 samples;
- aggregate complete bytes regress less than 3%;
- aggregate squared error regresses less than 1%;
- maximum error remains within the existing quantizer bound; and
- results are broken down by sample and bit depth.

## Result

Strict Clippy and formatting pass. The exact accounting test confirms that a
17-row tile becomes two independently modeled payloads, charges one five-byte
boundary, and sums payload/error values exactly.

| Sample | Depth | Complete-byte delta | SSE delta | Maximum error | Maximum band |
|---|---:|---:|---:|---:|---:|
| HDR gradient | 10 | +2.5281% | +0.0000% | 4 | 4,096 |
| Precision motion | 10 | +2.4109% | +0.0054% | 4 | 4,096 |
| Precision UI | 12 | +9.7065% | +0.0015% | 16 | 4,096 |
| Precision motion | 16 | +14.9865% | +0.0000% | 256 | 4,096 |
| **Aggregate** | mixed | **+4.9865%** | **+0.000002%** | **256** | **4,096** |

The reconstruction-error gate passes by a wide margin, but the aggregate
complete-byte gate fails. Tile-wide clamp-gradient payloads total 26,476,497
bytes; independent bands plus controls total 27,796,740 bytes.

Artifact:
`artifacts/exp0103-predictor-bands.tsv`
(`7d8971f04359cbd0ad97540fa4f796ab92e8fc3a7220fb15c8475234dd4f84dd`).

## Decision

Reject a fixed 16-row predictor boundary as a general format default. It is
viable for the tested 10-bit content but disproportionately taxes compact
12/16-bit payloads. The essentially unchanged SSE shows that boundary quality
is not the obstacle; repeated entropy modes, lengths, alignment, and worse
first-row residuals are.

Retain 16 rows as the maximum-parallelism endpoint for a predeclared
16/32/64/128-row model. A content-adaptive band height would require explicit
signaling and a selector with complete-byte costs; a fixed replacement would
require an unseen validation corpus under the methodology's fitted-constant
policy.

## References

- [Research 0037](../research/0037-parallel-hardware-friendly-codecs.md)
- [EXP-0100](EXP-0100-parallel-serialization-budget.md)
- [EXP-0102](EXP-0102-four-lane-rice-shard-model.md)
