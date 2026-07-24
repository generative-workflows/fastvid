# EXP-0060 — Fixed-gradient speed tier

Status: **ACCEPTED**

## Classification

**Exploration for the vacant speed frontier role** — trade a modeled amount of
spatial compression for a simpler fixed predictor and no encoder search.

## Hypothesis

A version-2 encoder using fixed clamp-gradient prediction for intra tiles and
fixed temporal prediction for inter tiles will exceed the balanced line's
encode throughput by at least 5% on a focused intra workload. Complete
EXP-0047 evidence predicts a bounded spatial size cost, making the candidate a
potential non-dominated speed point rather than a replacement for balanced.

## Predeclared model evidence

Across EXP-0047's complete 8-bit rows, fixed clamp-gradient payload cost
relative to fixed Paeth is:

- q90: 130,301,752 / 120,757,559 = **1.07904x**;
- q100: 196,199,844 / 185,297,477 = **1.05884x**.

The candidate therefore knowingly exchanges roughly 6–8% spatial payload for
a simpler kernel. Temporal residual construction and payloads are unchanged.

## Modification

Starting from practical version-2 source commit `4ad0318`:

- intra tiles directly encode clamp-gradient residuals and signal
  `PREDICT_CLAMP_GRADIENT`;
- inter tiles directly encode temporal residuals and signal
  `PREDICT_TEMPORAL` when the existing frame-level activity gate prefers
  temporal; high-motion fallback directly uses clamp-gradient;
- skip all per-tile multi-predictor evaluation, reconstruction candidates,
  squared-error tie breaking, and candidate allocations;
- keep the decoder and every specified prediction mode unchanged.

The first implementation is an isolated candidate build. It enters production
source only if it passes the frontier gate.

## Test

Fast feedback:

1. run q100 exactness and q90 error-bound tests;
2. alternate six candidate/balanced trials on 24-frame noisy-camera q90 GOP 1;
3. require deterministic bytes and metrics.

If the focused gate passes, run the four-case automated frontier protocol and
regenerate the frontier graph. Confirm video temporal sizes separately.

## Gate

- focused encode throughput at least 5% above balanced;
- no focused decode regression greater than 1%;
- byte increase consistent with the predeclared fixed-predictor model;
- q100 exact reconstruction and unchanged q90 error/quality bounds;
- candidate is non-dominated on the standard frontier matrix.

## References

- [Research 0026](../research/0026-paeth-data-dependency-kernel.md)
- [EXP-0047](EXP-0047-compatible-predictor-oracle.md)
- [EXP-0057](EXP-0057-automated-pareto-frontier.md)
- [EXP-0058](EXP-0058-frontier-speed-profile.md)

## Results

The focused noisy-camera q90 GOP-1 run used one warm-up and six alternating
trials:

| Variant | Encoded bytes | Ratio | Encode MP/s | Decode MP/s |
|---|---:|---:|---:|---:|
| balanced | 32,630,454 | 3.050304x | 38.8000 | 54.8985 |
| fixed gradient | 36,461,579 | 2.729799x | 52.4075 | 60.5475 |

The candidate improved encode by **35.07%** and decode by **10.29%**, while
increasing bytes by 11.74%. PSNR (`49.875764` dB), block SSIM (`0.99353615`),
and maximum error (`1`) were identical. The corpus-wide model predicted the
direction correctly; this noisy clip has a larger-than-average rate cost.

The eight-trial standard four-case matrix produced:

| Slot | Compression | Encode MP/s | Decode MP/s | Playback bitrate |
|---|---:|---:|---:|---:|
| speed | 13.353556x | 122.165817 | 145.108612 | 68.853922 Mb/s |
| balanced | 14.503542x | 96.901071 | 102.748668 | 63.394493 Mb/s |
| practical compression | 24.547776x | 29.138685 | 134.779533 | 37.455311 Mb/s |
| maximum compression | 33.613405x | 24.321670 | 97.109572 | 27.353510 Mb/s |

The candidate is non-dominated: it is 26.07% faster than balanced to encode
and 41.23% faster to decode at a 7.93% lower compression ratio. It is also
7.66% faster to decode than the previous decode leader. q100 exactness, the
q90 quantizer error bound, and high-motion spatial fallback tests passed.

The q90 GOP-12 single-frame-access confirmation covered all six benchmark
videos, four alternating trials, and target frames
`0,1,6,11,12,13,18,23`. Across 192 rows per variant:

- median access latency fell from 78.1865 ms to 64.5485 ms (**-17.44%**);
- median useful throughput rose 11.25%;
- median decoded-work throughput rose 36.42%;
- dependency-frame counts were identical;
- median bytes read rose 7.55%, the expected rate tradeoff.

Every target position improved, from 3.79% at target 6 to 18.80% at target 0.

## Artifacts

- focused A/B: `artifacts/exp0060-fixed-gradient-ab.tsv`
  (`0f96498c6e1578fdfcd0eee6bfaad4539e3b00deb5a43a5af8048bc807f0bfdc`);
- frontier matrix: `artifacts/exp0060-frontier.tsv`
  (`cf31e57828baa7282c86157be36e0ed7353934e1182081d817708a2967c868e4`);
- access A/B: `artifacts/exp0060-access-ab.tsv`
  (`c37b3859a28128d1a7ffe30e69f9e11f81fa8c7d27951b9ec595c84354f4b184`);
- reproducible source delta:
  `artifacts/frontier/exp0060-speed.patch`
  (`3a8b569d0292d3107e5e1b99fc885f444585131412150bfd3182fc451005c1f9`),
  applied to commit `4ad0318`;
- preserved binary: `artifacts/frontier/fastvid-speed-exp0060`
  (`f8e6bb69d7cf52b4531210e7423ec75a5626549ac1bacc964c1e123ca2bde8f7`).

## Decision

**Accepted as the speed frontier.** The candidate passes the focused,
standard-matrix, correctness, and random-access gates. Its larger streams are
an explicit rate/throughput tradeoff, not a regression hidden inside the
balanced role. Preserve all decoder modes and retain the exhaustive selector
for the compression roles.
