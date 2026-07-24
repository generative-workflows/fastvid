# EXP-0052 — 16-bit temporal decode guard

Status: **ACCEPTED**

## Classification

**Exploration of the compression/access frontier** — trade a bounded amount
of 16-bit inter-frame compression for practical decode and random-access
performance.

## Hypothesis

Preferring the previous-frame predictor on 16-bit inter tiles will remove the
serial Paeth zero-run workload responsible for EXP-0049's 59% decode
regression, while the larger combined corpus retains EXP-0048's 8% complete
stream saving.

## Modification

- For 16-bit inter frames, use temporal prediction instead of searching
  spatial predictors.
- Keep the staged spatial selector for 16-bit intra frames.
- Keep the exact staged selector unchanged for 10/12-bit and the exhaustive
  selector unchanged for 8-bit.
- Do not alter the bitstream or decoder.

This deliberately gives up the 16-bit motion stream's inter-predictor space
gain. It tests whether the production compression frontier should favor
bounded decode latency while retaining the format's maximum-compression
capability as a separate preserved variant.

## Correctness tests

- q100 exactness and lossy error bounds remain unchanged.
- Candidate mode counts return to temporal prediction on 16-bit inter tiles.
- All release tests, strict Clippy, formatting, and Lean pass.

## Measurement

1. Run focused six-trial 16-bit q90/q100 video A/B.
2. Run single-frame access on the 16-bit video.
3. Recompute complete-corpus bytes with the guarded 16-bit rows.

## Acceptance gate

- 16-bit q90/q100 decode and access regress no more than 10%.
- Combined complete-stream savings remain at least 8%.
- Eight-bit and 10/12-bit sizes remain unchanged.
- Quality and error bounds do not regress.

## Result

The six-trial high-bit video confirmation measured:

- 16-bit q90: identical to the balanced baseline size, 15.67% lower encode
  throughput, and 1.19% higher decode throughput;
- 16-bit q100: 4.04% fewer bytes, 8.53% lower encode throughput, and 7.62%
  lower decode throughput;
- 10-bit q90/q100: sizes unchanged from EXP-0051, with decode regressions of
  6.88% and 5.32%.

The guarded candidate replaces the maximum-compression 16-bit rows while
leaving the measured 8/10/12-bit matrix unchanged:

- baseline complete bytes: 761,576,255;
- guarded candidate complete bytes: 687,860,094;
- change: 73,716,161 fewer bytes, or 9.68%.

The four-trial warm-cache single-frame-access comparison against the
maximum-compression EXP-0051 binary measured:

- q90 access-time geometric mean: 42.12% lower;
- q90 target-23 access time, requiring 11 dependency frames: 63.79% lower;
- q100 access-time geometric mean: 4.15% lower;
- every q100 target remained within the 10% access gate.

Keyframe access was unchanged within 1.5%, isolating the improvement to
inter-frame dependency decoding. The expected tradeoff is more encoded bytes
read: q90 target 23 rises from 1,548,469 to 2,151,768 bytes, while the much
faster temporal copy path dominates elapsed time.

Artifacts:

- `artifacts/exp0052-highbit-video-ab.tsv`
  (`026e523d4f50785e496483127f9649db6aa18d84ca947452a52101e62949e29d`);
- `artifacts/exp0052-16bit-access-ab.tsv`
  (`a011a3e0ce7a706d937d71a179727c00337eada4a7ccc417dc34266371a93d8e`).

Access-capable comparison binaries:

- maximum compression:
  `658ede288ae519da52bea3c0a5d8ccbd73f71047621a24996a3d4fbfa53ab340`;
- guarded compression:
  `1235c7e82cf34fdddf5c341a5c17d265687368092174d175db709f22b17131c9`.

## Decision

Accept as the practical compression frontier. The temporal guard resolves the
16-bit decode and access regressions, retains 9.68% combined complete-stream
savings, and preserves quality. Keep EXP-0051 separately as a
maximum-compression variant for workflows willing to accept serial spatial
zero-run decoding.
