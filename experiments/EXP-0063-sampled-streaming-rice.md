# EXP-0063 — Sampled streaming Rice

Status: **REJECTED**

## Classification

**Speed-frontier exploitation with a new entropy-path heuristic** — remove
the measured residual buffer/histogram/finalization pass while retaining the
existing version-2 zero-run and fixed-Rice syntax.

## Hypothesis

A deterministic sparse-row proxy can select zero-run versus Rice and the Rice
parameter accurately enough to:

- improve focused speed-tier encode throughput by at least 8%;
- keep focused encoded bytes within 5%;
- preserve decoded samples and every quality metric exactly; and
- keep the four-case frontier compression-ratio loss below 5%.

## Modification

On the isolated EXP-0060 speed source:

1. Sample rows across the full tile width.
2. For temporal prediction, use the exact source/reference residual.
3. For fixed clamp-gradient prediction, use source neighbors as a cheap proxy
   for the real reconstructed-neighbor residual.
4. Compute complete sampled zero-run cost and all current Rice costs.
5. Signal the selected existing entropy mode.
6. During the single real causal reconstruction pass, write folded residuals
   directly to the selected writer.

Do not allocate the per-tile folded-residual vector or 511-bin production
histogram, and do not rescan a completed tile. The sampled estimator is stack
state only. Decoder and bitstream syntax remain unchanged.

Three implementations were screened:

1. every sixteenth row, streaming both zero-run and Rice;
2. the middle row only, streaming both modes; and
3. the middle row, streaming Rice but falling back to the established exact
   accumulator whenever the estimate selects zero-run.

The third hybrid isolates the measured opportunity without replacing the
fast zero-run path used by synthetic graphics and UI material.

## Test

1. Run q100 exactness, q90 error-bound, malformed-stream, high-motion fallback,
   and individual-tile decode controls.
2. Alternate at least six focused noisy-camera q90 GOP-1 trials against the
   preserved EXP-0060 speed binary.
3. If the focused gate passes, run the standard three-version frontier matrix
   and q90 GOP-12 access confirmation.
4. Report entropy-mode counts and content-specific byte outliers.

## Gate

- focused encode +8% or better;
- focused bytes no worse than +5%;
- no focused decode regression above 1%;
- exact quality/decoded-sample invariance;
- standard-matrix compression ratio no worse than -5%;
- remains non-dominated against practical and maximum compression.

## References

- [Research 0027](../research/0027-streaming-rice-parameter-selection.md)
- [EXP-0060](EXP-0060-fixed-gradient-speed-tier.md)
- [EXP-0062](EXP-0062-speed-tier-entropy-profile.md)

## Results

The first two variants improved the focused noisy-camera case but regressed
the standard matrix. Direct zero-run emission was the cause:

| Variant | Focused encode change | Focused byte change | Matrix speed encode |
|---|---:|---:|---:|
| every 16th row, direct both modes | +25.98% | +0.075% | 110.30 MP/s |
| middle row, direct both modes | +29.38% | +0.267% | 112.34 MP/s |
| middle row, Rice-only hybrid | +14.88% | +0.187% | 123.76 MP/s |

The Rice-only hybrid exactly retained the baseline entropy-mode counts in the
four-case screen: the camera case used 216 Rice tiles, while the cuts, grid,
and UI cases used 864, 765, and 360 zero-run tiles respectively. It therefore
removed the synthetic/UI collapse. Its six-trial standard frontier result was:

| Slot | Compression | Encode MP/s | Decode MP/s | Playback bitrate |
|---|---:|---:|---:|---:|
| speed candidate | 13.347305x | 123.761117 | 147.817878 | 68.886166 Mb/s |
| current speed | 13.353556x | 122.803921 | 147.634583 | 68.853922 Mb/s |

That is only **+0.78% encode throughput** for a **-0.05% compression-ratio**
change. The decoder was unchanged; its small measured difference is noise.
The hybrid missed the predeclared +8% speed gate, so no access benchmark was
warranted.

The release test suite passed 42 of 45 tests. The three failures compare the
speed policy against the maximum-compression predictor oracle or legacy mode
choice; all codec correctness controls passed, including exact q100,
bounded-error q90, high-motion fallback, individual-tile decode, Rice
round-trip, and malformed-stream rejection.

## Artifacts

- every-sixteenth-row focused A/B:
  `artifacts/exp0063-focused-ab.tsv`
  (`b7f883db4dad6c2c09a133e4f7b8b869817a21bc4228048f29718031c35c68c7`);
- every-sixteenth-row matrix:
  `artifacts/exp0063-frontier.tsv`
  (`b808e29e48100f040892958791955d301633371ea45591c1c64e587617ab91c7`);
- middle-row focused A/B:
  `artifacts/exp0063-middle-row-focused-ab.tsv`
  (`53617ab4125d13693feb8b38d88fd122145576ad7716345d773c587636e6888c`);
- middle-row matrix:
  `artifacts/exp0063-middle-row-frontier.tsv`
  (`023e1ec1575742ad703cc3ce097f4c31ee1a5f7cced3ab71c6dba2a31341ec86`);
- Rice-only hybrid A/B:
  `artifacts/exp0063-hybrid-ab.tsv`
  (`95ecbff15433c2f27911868b94e25750044ba3d08c9d41e9383e211c80fdb474`);
- Rice-only hybrid matrix:
  `artifacts/exp0063-hybrid-frontier.tsv`
  (`c92ed9857885f9f73a753af63271d57f184b6129a9ef7816f82350a8c320044a`);
- exact combined source delta from commit `4ad0318`:
  `artifacts/exp0063-hybrid.patch`
  (`82f6f61534e8902c0a02ee2d6d9c7bf78e53879cbae762f91a29df49cf694182`);
- hybrid binary:
  `/tmp/fastvid-exp0063.wd5Uve/target/release/fastvid`
  (`c279b428ad2163dfe096cc9156534052e21b224a4217899572fee3f2ae0b4888`).

## Decision

**Rejected.** Sparse sampling is worthwhile on Rice-heavy camera tiles, but a
universal estimator pass cannot clear the whole-corpus speed gate. Keep the
existing speed frontier. A future attempt should reuse statistics already
needed by prediction or specialize only after a nearly free Rice-heavy
classifier; it should not add a second universal tile walk.
