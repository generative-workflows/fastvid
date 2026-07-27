# EXP-0152 — Version-5 full-frame shard order-0 model

Status: **ACCEPTED**

Date: 2026-07-27

## Hypothesis

The fully charged shard-local order-0 candidate accepted for first-frame
screening in EXP-0148 will retain a material complete-stream byte advantage on
all 350 corpus-v4 frames, including the newly added real-world 4K motion.
Evaluating every frame and joining quality by `(sample, frame, quality)` will
give a sounder bound for the amended minimum-per-frame XPSNR target than the
first-frame result.

## Modification

This is a model-only experiment. Extend `v5_entropy_model` and its sweep and
summary scripts to:

- process every declared frame rather than only frame zero;
- make sample/quality cells resumable without duplicate rows;
- charge normalized frequency tables, final rANS states, body byte rounding,
  shard headers, the existing tile directory, and stream header;
- independently retain the smaller current or order-0 representation in every
  4,096-symbol shard; and
- join XPSNR using the full-frame EXP-0151 artifact.

No codec format, encoder selector, decoder, or CUDA kernel changes.

## Test

1. Complete q80/q85/q90/q95/q100 analysis for all 350 corpus-v4 frames.
2. Reject duplicate `(sample, frame, quality)` rows and require exactly 350
   rows at each quality.
3. Report complete-stream compression and savings for the full corpus and
   1920x1080 subset.
4. Report fixed-quality minimum frame-level luma XPSNR and an optimistic
   per-frame quality oracle satisfying `>50 dB`.
5. Treat the model as actionable if it saves at least 10% of complete encoded
   bytes on the quality oracle. It satisfies the simultaneous rate/quality
   screen only if the fully charged oracle also exceeds 15x compression.

## Results

Artifact:
`artifacts/exp0152-v5-entropy-model.tsv`

SHA-256:
`a6d14299349bdf5227912e3caf8750af333dd2372df1e5962388193296f1fe86`

The artifact contains 1,750 unique rows: all 350 frames at each of five
qualities. Coverage exactly matches EXP-0151.

| Scope | Q/control | Current | Charged oracle | Saving | Minimum Y XPSNR |
|---|---:|---:|---:|---:|---:|
| corpus | 80 | 10.532200x | 11.807076x | 10.798% | 34.4485 dB |
| corpus | 85 | 9.516107x | 10.470392x | 9.114% | 36.8623 dB |
| corpus | 90 | 8.225066x | 8.879407x | 7.369% | 40.0209 dB |
| corpus | 95 | 6.519944x | 6.881465x | 5.254% | 45.3638 dB |
| corpus | 100 | 3.923606x | 6.332831x | 38.043% | exact |
| corpus | frame oracle | 7.269969x | 8.127258x | 10.548% | 50.0476 dB |
| 1080p | frame oracle | 5.818175x | 6.688786x | 13.016% | 50.0529 dB |
| 4K | frame oracle | 7.472441x | 8.198322x | 8.854% | 50.1507 dB |

At the quality-qualified oracle, order-0 wins 729,620/856,288 shards and
passes the predeclared 10% actionability gate. It still reaches only 8.13x,
far below 15x.

Durable reports:

- `benchmarks/v5-entropy-model-v4.md`
- `benchmarks/v5-entropy-model-v4-summary.tsv`

## Decision

Accepted as a complete-byte entropy bound. The first-frame finding
generalizes, especially for exact coding, but an order-0-only v6 would be
misaligned with the simultaneous target. Defer payload implementation and
combine this entropy family with the temporal/finer-quantizer direction
screened by EXP-0153.

## References

- [EXP-0148](EXP-0148-v5-shard-order0-model.md)
- [EXP-0151](EXP-0151-corpus-v4-full-frame-feedback.md)
- [Research 0024](../research/0024-finite-block-ans-entropy-models.md)
- [Research 0030](../research/0030-entropy-decode-consumer-fusion.md)
