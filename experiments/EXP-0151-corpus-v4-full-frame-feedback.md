# EXP-0151 — corpus-v4 full-frame rate-quality feedback

Status: **ACCEPTED**

Date: 2026-07-27

## Hypothesis

The four raw/native-master 4K additions will change both the weighted rate
boundary and the minimum-per-frame XPSNR distribution relative to corpus-v3.
Measuring all 350 frames at every current 10-bit quantizer step will establish
the authoritative corpus-v4 quality constraint before an entropy or quantizer
format change is implemented.

## Modification

Run the EXP-0149 per-frame harness on all 28 corpus-v4 codec samples at
q80/q85/q90/q95/q100. Preserve encoded bytes and Y/U/V XPSNR for each frame,
plus fixed-quality, per-sample adaptive, and optimistic per-frame adaptive
summaries. Report the 170-frame 4K slice separately.

## Test

- Require 350 unique rows at each quality and 1,750 total.
- Require q100 exactness for every frame.
- Verify all 254 inherited corpus-v3 frame rows are byte/metric-identical.
- Report the minimum frame and source at each setting; no averaging may hide a
  quality failure.

## Gate

Accept the measurement if all rows and controls pass. Use the resulting
minimum-per-frame boundary for the next quantizer/entropy experiment.

## Results

Artifact:
`artifacts/exp0151-v5-frame-quality.tsv`

SHA-256:
`c41b327e84adc57f266db5a8d5a317f67c19fead5c0158b15e608678456577c8`

The artifact has 1,750 unique `(sample, frame, quality)` rows, exactly 350 at
each quality. All 350 q100 frames reconstruct exactly. All 1,270 inherited
corpus-v3 rows match EXP-0149 field-for-field.

| Scope | Q/control | Compression | Minimum frame Y XPSNR | Passing frames |
|---|---:|---:|---:|---:|
| corpus | 80 | 10.532200x | 34.4485 dB | 43/350 |
| corpus | 85 | 9.516107x | 36.8623 dB | 70/350 |
| corpus | 90 | 8.225066x | 40.0209 dB | 137/350 |
| corpus | 95 | 6.519944x | 45.3638 dB | 270/350 |
| corpus | 100 | 3.923606x | exact | 350/350 |
| corpus | per-frame oracle | 7.269969x | 50.0476 dB | 350/350 |
| 1080p | per-frame oracle | 5.818175x | 50.0529 dB | 130/130 |
| 4K | per-frame oracle | 7.472441x | 50.1507 dB | 170/170 |

The expanded native-master 4K footage materially lowers the aggregate
compression frontier compared with the v3 first-frame screening. Even the
optimistic current-format per-frame quality oracle misses 15x by more than
2x. The coarse step ladder also forces 80/350 frames to lossless q100.

Durable reports:

- `benchmarks/v5-frame-quality-v4.md`
- `benchmarks/v5-frame-quality-v4-summary.tsv`

## Decision

Accepted as the authoritative corpus-v4 intra-frame baseline. Entropy coding
alone cannot plausibly close the 7.27x-to-15x gap; after EXP-0152 quantifies
the entropy bound, the next format experiment must combine finer quantization
with a stronger source of redundancy such as bounded temporal prediction.

## References

- [EXP-0149](EXP-0149-v5-full-frame-quality-audit.md)
- [EXP-0150](EXP-0150-corpus-v4-uvg-xiph.md)
- [Evaluation methodology](../EVALUATION_METHODOLOGY.md)
