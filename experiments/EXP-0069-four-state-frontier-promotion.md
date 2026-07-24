# EXP-0069 — Four-state frontier promotion

Status: **ACCEPTED**

## Classification

**Frontier exploitation and evaluation hardening** — replace the preserved
maximum-compression binary only if the accepted EXP-0068 candidate remains
non-dominated in a balanced three-version run.

## Hypothesis

The budgeted four-state rANS candidate will remain within the frontier's 1%
encoded-byte tolerance of EXP-0055 while materially improving decode or
access beyond the 5% timing tolerance. Practical-compression and speed slots
will remain distinct, so only the maximum slot should change.

## Modification

- Preserve the exact release binary from source commit `36b1d20`.
- Replace the maximum-compression entry in the human and machine-readable
  frontier registries.
- Run all three hash-validated binaries under the six-trial cyclic-order
  fast-feedback matrix and regenerate the automatic summary and graph.
- Refresh the README's 18-sample q90/q100 current snapshot.
- Normalize cross-version benchmark outputs by metric name so diagnostic
  columns added by newer binaries cannot shift values positionally.

## Test and gate

- exact binary hash matches the registry;
- all three slots have four cases and trials 1 through 6;
- graph self-test passes and repeated generation is deterministic;
- promoted ratio remains within 1% of EXP-0055;
- the candidate's EXP-0068 controlled A/B improvement exceeds 5% for decode
  and access;
- README values come from all 18 codec-track samples with warm-up and two
  recorded trials per q90/q100 cell;
- retain exactly three active frontier versions.

## References

- [EXP-0057](EXP-0057-automated-pareto-frontier.md)
- [EXP-0055](EXP-0055-modeled-rans-selector.md)
- [EXP-0068](EXP-0068-four-state-rans.md)

## Result

The first frontier aggregation was invalid despite a successful benchmark:
the harness reused an older binary's positional header, so the new
`tile_width` and `tile_height` diagnostics shifted every later field. It
reported an impossible 9,577,548x ratio and 512 encoded bytes. Named-column
normalization fixed the root cause; the bad summary and SVG were discarded.

The corrected balanced frontier is:

| Slot | Compression | Encode MP/s | Decode MP/s | Playback bitrate |
|---|---:|---:|---:|---:|
| practical compression | 24.547776x | 28.563878 | 131.605265 | 37.455311 Mb/s |
| maximum compression | 33.588694x | 24.116615 | 99.137737 | 27.373634 Mb/s |
| speed | 13.353556x | 118.410117 | 141.260520 | 68.853922 Mb/s |

Against the prior maximum aggregate of 33.613405x, the promoted binary gives
up only 0.074% ratio. EXP-0068's same-run A/B established +5.31% standard
decode and +7.16% access throughput, satisfying the material timing gate.
Neither practical compression nor speed is dominated by the new maximum
point because both retain distinct throughput/rate roles.

The refreshed 18-sample README snapshot is:

| Quality | Ratio | Encode MP/s | Decode MP/s | Y quality |
|---:|---:|---:|---:|---:|
| 90 | 9.191348x | 16.429889 | 49.211250 | 49.908095 dB |
| 100 | 5.983214x | 15.947889 | 47.009722 | exact |

The matrix contained 72 recorded rows and the README artifact contained 72
recorded rows plus its header. Every preserved binary hash validated, the
graph self-test passed, and normalized rows had stable encoded sizes.

## Artifacts

- preserved binary:
  `artifacts/frontier/fastvid-rans4-exp0068`
  (`d4d7edaf68a67601f753652757d62bcc49ff237e9ef0954ad0174ddc45322a14`);
- balanced raw frontier:
  `artifacts/frontier-fast-feedback.tsv`
  (`c7771bc71d8b5c8f286d2ab04c55ddea8b7108f429dc0567a0fad5ca55251b1e`);
- durable summary:
  `benchmarks/frontier-summary.tsv`
  (`4b99fd0ba17aad09d1c2cdb82e0af519a26a468a9f70323f70f4b03366b559ae`);
- frontier graph:
  `benchmarks/frontier.svg`
  (`e73f51fe3b7fa6ab57db61e674fd9d90abc7a2e3d20021f72f2f968b1edeb809`);
- README corpus snapshot:
  `artifacts/exp0068-readme-corpus.tsv`
  (`30b28472a5490928295d121b548e70bc1264d04b47316f82923b02322c45f28b`).

## Decision

**Accepted.** Replace EXP-0055 with EXP-0068 in the maximum-compression slot.
Keep the practical-compression and speed slots unchanged, preserving exactly
three active versions. Cross-version benchmark scripts now normalize named
metrics as required by evaluation methodology version 10.
