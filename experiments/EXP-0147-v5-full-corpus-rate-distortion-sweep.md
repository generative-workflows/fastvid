# EXP-0147 — v5 full-corpus rate-distortion sweep

Status: **ACCEPTED**

## Hypothesis

Q90 achieves 11.6875x aggregate compression with a minimum luma XPSNR of
51.9589 dB, leaving quality headroom but missing the >15x rate target. Measuring
the five distinct 10-bit quantizer steps around this operating point will show
whether quantizer calibration alone can satisfy >15x and >50 dB simultaneously
or whether prediction/entropy changes are required.

## Modification

Add a deterministic first-frame corpus sweep for q80, q85, q90, q95, and q100
(10-bit steps 17, 13, 9, 5, and 1). Record encoded bytes, weighted compression,
XPSNR per plane, minimum quality, and aggregate/per-sample target passes for all
24 codec-track samples and the 15-sample 1080p slice.

## Test

Run the sweep against checksummed corpus-v3 with the release Rust reference.
Accept the measurement as the next decision boundary if all 120 cells complete,
q100 is exact/infinite-XPSNR, and repeated q90 rate/quality values match the
existing deterministic feedback rows.

## Result

All 120 cells completed. Q100 was exact for 24/24 samples, and q90 reproduced
the existing deterministic 11.687517x aggregate ratio and 51.9589 dB minimum
luma XPSNR. No fixed setting passed both gates: q85 reached 13.377732x with
23/24 samples above 50 dB; q80 reached 14.678455x but only 18/24 passed quality
and the minimum fell to 46.2430 dB.

An oracle choosing the coarsest tested step above 50 dB independently for each
sample selected q80 for 18, q85 for 5, and q90 for 1. Even this nonimplementable
oracle reached only 14.352886x at a 50.0476 dB minimum. The 1080p oracle reached
10.215203x. Quantizer selection alone therefore cannot meet both targets on
this corpus.

The raw 120-row table SHA-256 is
`cd1b665b9ee5593a56d112e1546eb9ac0fd5565403a6c744c01df0fa88ab481d`.

## Decision

Accept the sweep as the compression decision boundary. Preserve q90 as the
uniform high-quality control and pursue prediction/entropy efficiency; do not
claim that retuning the existing coarse quality mapping can close the gap.

## References

- [EXP-0139](EXP-0139-cuda-feedback-loop.md)
- [EXP-0146](EXP-0146-cuda-device-metadata-parse.md)
- [Evaluation methodology](../EVALUATION_METHODOLOGY.md)
