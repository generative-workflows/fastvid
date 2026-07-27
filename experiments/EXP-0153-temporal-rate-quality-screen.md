# EXP-0153 — Temporal rate-quality feasibility screen

Status: **REJECTED**

Date: 2026-07-27

## Hypothesis

The greater-than-2x rate gap measured by EXP-0151 cannot be closed by a
shard-local entropy replacement alone, but bounded previous-frame prediction
can remove enough redundancy from the 24-frame corpus sequences to make the
simultaneous `>15x` compression and `>50 dB` minimum frame-level luma XPSNR
target plausible. The existing Rust version-2 reference path provides an exact
feasibility control before temporal syntax is designed for the GPU-oriented
format.

## Modification

Add a measurement harness around the existing Rust
`encode16_with_reference`/`decode16_with_reference` implementation. Use GOP 12
for multi-frame samples and GOP 1 for stills, preserve reconstructed-reference
propagation, and retain every per-frame encoded size and decoded plane for
FFmpeg XPSNR. Test q95 and q100 first; expand to a finer quantizer only if
neither establishes the rate/quality boundary.

This experiment does not promote version 2, change version 5, or claim CUDA
suitability. It measures the value of temporal redundancy under the existing
random-access depth.

## Test

1. Process all 350 corpus-v4 frames with no omitted sequence.
2. Require exact reconstruction for every q100 frame.
3. Score XPSNR independently per frame and report corpus, 1080p, and 4K
   minimum luma values.
4. Aggregate complete per-frame stream bytes, including repeated headers and
   directories.
5. Report GOP-12 single-frame dependency depth and retain the existing
   maximum of 11 prerequisite frames.

## Gate

Treat temporal coding as the next format direction if it either passes the
simultaneous target or improves the quality-qualified compression ratio by at
least 50% relative to EXP-0151's 7.269969x intra-frame oracle. Otherwise,
investigate a transform or multi-frame predictor before adding temporal syntax
to the CUDA format.

## Results

Artifact:
`artifacts/exp0153-v2-temporal-quality.tsv`

SHA-256:
`970efd9a94c8acd8c0a4d21591ccae987330bf42fae127fdbb70e32b68561699`

The artifact contains 700 unique rows, exactly 350 at q95 and q100. Every
q100 frame is exact. Forty-two of 350 frames are keyframes under GOP 12 for
sequences and GOP 1 for stills.

| Scope | Q/control | Compression | Minimum frame Y XPSNR | Passing frames |
|---|---:|---:|---:|---:|
| corpus | 95 | 6.632469x | 45.4098 dB | 270/350 |
| corpus | 100 | 3.982847x | exact | 350/350 |
| corpus | sample-adaptive | 5.890771x | 51.7948 dB | 350/350 |
| 1080p | sample-adaptive | 5.242252x | 51.7948 dB | 130/130 |
| 4K | sample-adaptive | 5.922917x | 52.6240 dB | 170/170 |

The sequence-consistent control selects q95 for 22 samples and q100 for six.
Its 5.89x corpus result is worse than both EXP-0151's 7.27x optimistic
per-frame intra bound and its 6.36x sequence-consistent intra control.

The existing encoder uses frame-global zero-motion temporal prediction only
when mean absolute luma change is below its activity threshold. This helps
nearly static material but rejects or poorly represents the newly added
real-world motion; repeated frame headers are not the limiting factor.

Durable reports:

- `benchmarks/v2-temporal-quality-v4.md`
- `benchmarks/v2-temporal-quality-v4-summary.tsv`

## Decision

Rejected. Uncompensated previous-frame prediction with a frame-global gate
does not pass the 50% improvement gate and is not the temporal syntax to carry
into the GPU format. The next compression model must test tile/block-local
motion compensation (with charged vectors and reconstructed-reference
propagation) or an equally strong multi-frame transform before changing the
bitstream.

## References

- [EXP-0015](EXP-0015-temporal-copy-corpus-confirmation.md)
- [EXP-0048](EXP-0048-tile-predictor-format.md)
- [EXP-0151](EXP-0151-corpus-v4-full-frame-feedback.md)
- [Research 0007](../research/0007-temporal-dpcm-gating.md)
- [Research 0010](../research/0010-single-frame-random-access.md)
