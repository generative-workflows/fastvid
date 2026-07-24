# EXP-0067 — Fused rANS reconstruction

Status: **REJECTED**

## Classification

**Maximum-frontier exploitation** — remove one allocation and complete memory
pass from the measured order-0 decode hot path without changing the stream.

## Hypothesis

Passing each decoded rANS residual directly to tile reconstruction will improve
focused maximum-tier decode throughput by at least 5%, with identical encoded
bytes and decoded samples. It will not regress standard-corpus maximum-tier
decode throughput by more than 1%.

## Modification

Refactor the existing scalar rANS parser/state machine to accept an inlined
fallible symbol consumer. Keep `decode_rans_symbols` as a vector-collecting
wrapper for model tooling and unit tests. In production tile decode:

1. allocate only the final output tile;
2. decode one folded residual;
3. validate and reconstruct that sample immediately; and
4. retain all existing canonical-state and trailing-byte checks.

Do not alter tables, entropy state count, encoded bytes, prediction, or unsafe
code.

## Test

1. Run formatter, unit tests, and Clippy.
2. Compare baseline and candidate output streams and decoded output byte for
   byte on q100 and q90 controls.
3. Alternate at least six focused one-thread q90 GOP-1 trials on the
   24-frame 1920x1080 noisy-camera clip.
4. If the focused gate passes, run the standard maximum-frontier corpus matrix
   and random-access confirmation.

## Gate

- focused decode throughput at least +5%;
- focused encode throughput and bytes statistically/bitwise unchanged;
- standard-matrix maximum-tier decode no worse than -1%;
- exact stream and decoded-output identity;
- all malformed-stream checks pass.

## References

- [Research 0024](../research/0024-finite-block-ans-entropy-models.md)
- [Research 0030](../research/0030-entropy-decode-consumer-fusion.md)
- [EXP-0055](EXP-0055-modeled-rans-selector.md)
- [EXP-0066](EXP-0066-maximum-compression-profile.md)

## Result

All 54 release tests passed, including order-0 canonical-table rejection,
truncated/trailing payload rejection, q100 exactness, q90 error bounds, and
individual-tile decoding. Separate q90 and q100 one-frame controls confirmed
that baseline and candidate streams were byte-identical and that their decoded
YUV outputs were byte-identical.

The initial A/B harness used the preserved binary's older benchmark header,
which lacks the candidate's subsequently added `tile_width` and `tile_height`
columns. The twelve result rows were intact; the artifact was normalized by
adding those named columns and leaving them blank for the old binary before
any aggregation.

Six alternating focused trials averaged:

| Variant | Encoded bytes | Encode MP/s | Decode MP/s | Playback bitrate |
|---|---:|---:|---:|---:|
| preserved EXP-0055 | 29,518,163 | 13.250167 | 41.916000 | 236.145304 Mb/s |
| fused candidate | 29,518,163 | 13.370167 | 42.276333 | 236.145304 Mb/s |

The candidate's decode change was only **+0.86%**, while individual trial
ranges overlapped substantially (baseline 41.346–43.121 MP/s, candidate
41.239–43.470 MP/s). The apparent +0.91% encode difference is noise because
the encode implementation and output bytes are unchanged. The result misses
the +5% focused gate, so the standard matrix and access confirmation were not
warranted.

Artifacts:

- normalized alternating A/B:
  `artifacts/exp0067-focused-ab.tsv`
  (`f72a647e71d063b078bb14a604ed2c6238e2a9e092f26720bf59418f5617ee91`);
- exact rejected source delta:
  `artifacts/exp0067-fused-rans.patch`
  (`bb18d16e0d0002683e31b43e9a833def77013fc10fd4429ac79509a6fd27a560`);
- candidate binary:
  `target/release/fastvid`
  (`d786496144a4c63f2bded6e78755d63f6c5cf8710cc2fd0466af82b377878983`).

## Decision

**Rejected.** The temporary residual allocation and reread are real but not a
large enough whole-decode cost to justify a generic callback abstraction.
The production change was removed. A future order-0 speed attempt should
change the measured serial state cost—most plausibly through a byte-charged
multi-state format experiment—rather than only rearranging its output.
