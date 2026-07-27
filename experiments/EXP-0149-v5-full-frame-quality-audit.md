# EXP-0149 — v5 full-frame rate-quality audit

Status: **ACCEPTED**

## Hypothesis

The first-frame EXP-0147 sweep may overstate the quality boundary on motion
sequences. Measuring every one of the 254 core-corpus frames independently at
q80, q85, and q90 will determine the actual minimum per-frame luma XPSNR and
the rate cost of satisfying the amended greater-than-50 dB target.

## Modification

Add a deterministic all-intra full-frame sweep. Convert and encode every frame
of the fourteen still/ten 24-frame corpus-v3 samples, preserve exact encoded
bytes per frame, and record FFmpeg XPSNR Y/U/V values for every decoded frame.
The headline quality statistic is the minimum frame-level luma XPSNR; sequence
averages are diagnostics only.

## Test

- Require exactly 254 rows at each tested quality and 762 rows total.
- Cross-check every sample's frame count against the manifest.
- Reproduce the existing first-frame encoded bytes and XPSNR values.
- Report fixed-quality, per-sample adaptive, and optimistic per-frame adaptive
  rate-quality boundaries without allowing averages to hide a failing frame.

## Gate

Accept the audit when all rows complete and deterministic first-frame controls
match EXP-0147. Use its worst frame—not EXP-0147's first-frame minimum—as the
quality boundary for subsequent compression experiments.

## References

- [Research 0043](../research/0043-xpsnr-quality-metric.md)
- [EXP-0147](EXP-0147-v5-full-corpus-rate-distortion-sweep.md)
- [EXP-0148](EXP-0148-v5-shard-order0-model.md)
- [Evaluation methodology](../EVALUATION_METHODOLOGY.md)

## Result

All five quantizer points q80/q85/q90/q95/q100 completed for every one of the
254 corpus-v3 frames (1,270 unique rows). First-frame encoded bytes and XPSNR
matched EXP-0147 exactly. The full-frame boundary is materially worse than the
old first-frame panel:

| Q | Compression | Minimum frame Y XPSNR | Frames >50 dB |
|---:|---:|---:|---:|
| 80 | 14.688184x | 34.4485 dB | 35/254 |
| 85 | 13.356350x | 36.8623 dB | 62/254 |
| 90 | 11.660067x | 40.0209 dB | 63/254 |
| 95 | 9.436829x | 45.3638 dB | 174/254 |
| 100 | 5.735632x | infinite/exact | 254/254 |

The worst lossy frame at every fixed setting is in
`procedural-scene-cuts`; q95 still reaches only 45.3638 dB. A per-sample
adaptive control that satisfies every frame reaches 7.052671x. Even an
optimistic per-frame quality oracle reaches only 9.154450x at 50.0476 dB and
requires exact q100 for 80/254 frames.

The raw artifact SHA-256 is
`2ee31c43f6b6011c14e6ee5de8ae0e945ff49d3739ae9c9d5b2cdefda9c31d28`.

## Decision

Accept the audit and replace first-frame/sequence-average XPSNR with minimum
per-frame luma XPSNR as the authoritative quality gate. The current coarse
10-bit step ladder cannot approach >15x and >50 dB simultaneously; subsequent
work must improve quantizer granularity and prediction/entropy efficiency,
then validate every frame on corpus-v4.
