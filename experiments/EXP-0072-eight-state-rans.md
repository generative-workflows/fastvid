# EXP-0072 — Budgeted eight-state rANS

Status: **REJECTED**

## Classification

**Speed exploration** — widen the current independent-state scalar rANS
decoder to expose more instruction-level parallelism without changing
prediction, quantization, quality, tile geometry, or threading.

## Hypothesis

Eight interleaved rANS states will improve focused order-0 decode throughput
by at least 5% relative to the promoted four-state maximum-compression binary,
while a complete-byte selector keeps standard-corpus encoded size within 1%.

## Modification

- add a distinct version-3 entropy mode with eight final rANS states;
- retain scalar and four-state modes;
- select eight states only when its 28 bytes of additional final-state storage
  are no more than 0.5% of the modeled scalar payload;
- otherwise retain the existing four-state 12-byte budget and scalar fallback;
- decode eight symbols as one independent group before renormalization.

No default tile geometry or corpus-fitted global constant changes.

## Test

1. Exact round-trip, malformed-stream, truncation, and individual-tile tests.
2. Focused balanced A/B on an order-0-heavy sample.
3. Fast standard-corpus A/B if the focused speed gate passes.
4. Record complete bytes, encode/decode throughput, and stream mode counts.

## Gate

- at least 5% focused decode-throughput improvement;
- standard-corpus encoded-byte regression no more than 1%;
- no unexplained standard-corpus encode/decode regression over 5%;
- exact q100 reconstruction and unchanged bounded-loss error;
- only then consider format and frontier confirmation.

## Result

All 54 release library tests and both motion-model tests passed. Controls
covered exact q100 reconstruction, individual-tile equivalence, sparse and
extreme alphabets, truncated and trailing payloads, final-state validation,
and the new eight-state mode. The exact encoder sometimes differed by as much
as eight bytes from the scalar payload plus 28 state bytes because
renormalization boundaries also change; the candidate's actual complete
payload, rather than the nominal model alone, was measured.

Six one-thread q90 GOP-1 trials on the 24-frame
`noisy-camera-fourpeople-1920x1080-24f` sequence alternated the preserved
four-state frontier binary and the eight-state candidate:

| Variant | Encoded bytes | Encode MP/s | Decode MP/s | Playback bitrate |
|---|---:|---:|---:|---:|
| Four-state | 29,564,701 | 13.593667 | 47.858000 | 236.517608 Mb/s |
| Eight-state | 29,583,544 | 13.810667 | 47.968167 | 236.668352 Mb/s |

Eight states changed complete bytes by **+0.064%**, encode throughput by
**+1.596%**, and decode throughput by only **+0.230%**. Quality and tile mode
counts were identical. The decode result is far below the predeclared 5%
focused gate, so the standard-corpus slow tier was not run.

Artifacts:

- `artifacts/exp0072-rans8-focused.tsv`
  (`f842d12d558496ecd18ac7635f306e0185602ccfba0d059f5ccfdddd68dbc6be`);
- `artifacts/exp0072-rans8.patch`
  (`f01ec017a47fcc8c2b8a3f28d31e437686c85332408735e2ea5a2dcd5cd900f9`);
- `artifacts/fastvid-rans8-exp0072`
  (`d531464dc0ea93e64ccaeb7ef17d97231e42a780cf07c0e085e2b54704047ced`).

## Decision

Reject and remove the eight-state format mode. Four independent scalar states
already expose nearly all useful rANS instruction-level parallelism on this
host; doubling state count buys only 0.230% focused decode throughput while
increasing the stream. The preserved maximum-compression frontier remains
unchanged.

## References

- [Research 0030](../research/0030-entropy-decode-consumer-fusion.md)
- [Research 0031](../research/0031-modern-simd-rans-implementation.md)
- [EXP-0068](EXP-0068-four-state-rans.md)
- [EXP-0070](EXP-0070-promoted-rans-profile.md)
