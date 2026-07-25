# EXP-0104 — Predictor band-height ladder

Status: **ACCEPTED**

## Classification

**Predictor-format exploration** — rate/serialization Pareto model after the
fixed 16-row rejection.

## Hypothesis

Independent 32- and 64-row clamp-gradient bands should recover most of
EXP-0103's boundary rate while still reducing the tile-wide predictor span.
On the native high-bit q90 supplement:

- 64 rows should cap luma bands at 16,384 samples, regress aggregate complete
  bytes by less than 3%, and regress every sample by less than 5%;
- 32 rows should cap luma bands at 8,192 samples and regress aggregate bytes
  by less than 4%; and
- both should keep aggregate squared-error change below 1%.

## Modification

Generalize the exact EXP-0103 model to independently reconstructed
16/32/64-row bands. Each point uses the same clamp-gradient predictor,
quantizer, per-band zero-run/Rice selection, five-byte added-boundary charge,
and exact error accounting. No codec stream changes.

## Gate

- strict Clippy, formatting, and the existing exact boundary test pass;
- the declared span/rate/error thresholds pass;
- report every ladder point per sample/depth; and
- identify non-dominated points without selecting a new default from the
  development corpus.

## Result

Native high-bit q90:

| Sample | Depth | 16-row bytes | 32-row bytes | 64-row bytes |
|---|---:|---:|---:|---:|
| HDR gradient | 10 | +2.5281% | +1.0688% | +0.3440% |
| Precision motion | 10 | +2.4109% | +1.0510% | +0.3704% |
| Precision UI | 12 | +9.7065% | +4.1203% | +1.3191% |
| Precision motion | 16 | +14.9865% | +6.5277% | +2.3076% |
| **Aggregate** | mixed | **+4.9865%** | **+2.1657%** | **+0.7564%** |

Maximum luma spans are respectively 4,096, 8,192, and 16,384 samples.
Aggregate SSE changes are +0.0000022%, +0.0000017%, and +0.0000010%; maximum
errors remain at the existing quantizer bounds. Both 32- and 64-row points
pass their predeclared gates.

A representative 8-bit diagnostic covering a natural camera still, AI image,
noisy-camera video, UI animation, and 4K synthetic grid measured:

| Sample | 16-row bytes | 32-row bytes | 64-row bytes |
|---|---:|---:|---:|
| AI greenhouse | +3.983% | +1.947% | +0.559% |
| Camera cholla | +2.108% | +1.012% | +0.312% |
| Noisy camera | +3.084% | +1.501% | +0.618% |
| 4K grid | +1.357% | +0.563% | +0.315% |
| UI scroll | +37.426% | +12.506% | -0.822% |
| **Aggregate** | **+3.421%** | **+1.593%** | **+0.575%** |

The UI outlier confirms that dense restart metadata is content-sensitive;
64-row bands happen to improve it because independent entropy selection finds
cheaper substreams, but that is not assumed to generalize.

Artifacts:

- native ladder:
  `artifacts/exp0104-predictor-band-ladder.tsv`
  (`24f5369564ee1fd14b5d86392dc2aefb8a372c2890d3a794c4e2ccfd242df387`);
- representative core screen:
  `artifacts/exp0104-predictor-band-core-screen.tsv`
  (`9b56675138ae09ec9e0f11e5a89cc2f1054d683b9f0f5be9851f0702bf91e9e7`).

Strict release Clippy, formatting, and the exact boundary accounting test
pass.

## Decision

Accept 32- and 64-row bands as non-dominated format-model points:

- 32 rows offers a 4x predictor-span reduction for +2.166% native bytes;
- 64 rows offers a 2x reduction for +0.756% native bytes.

Do not select a default from these development results. Sixty-four rows is
the stronger first implementation candidate because it stays below +2.31%
on every native sample and below +0.82% in magnitude on each representative
core sample, but promotion requires a frozen validation corpus. The
16-row point remains useful only as a maximum-parallelism tradeoff and failed
the general default gate in EXP-0103.

## References

- [EXP-0103](EXP-0103-independent-predictor-bands.md)
- [Research 0037](../research/0037-parallel-hardware-friendly-codecs.md)
