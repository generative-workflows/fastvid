# EXP-0065 — Integer block-motion potential

Status: **REJECTED**

## Classification

**Exploration: temporal prediction** — a distinct video-compression branch
from tile geometry, spatial predictors, entropy models, and implementation
micro-optimization.

## Hypothesis

On 64x64 luma blocks of predicted frames, a luma-SAD-selected integer
translation from the immediately preceding source frame will reduce charged
folded-residual Rice bits by at least 15% versus co-located temporal
prediction on at least three of the six standard videos, including at least
one natural/camera-derived video and one animation/UI/procedural video.

## Modification

Add a read-only `motion_model` binary; do not change codec encoding or syntax.

For each non-key frame and 64x64 luma block:

1. search displacements `[-16, 16]` in four-pixel steps;
2. reject candidates whose reference rectangle leaves the frame;
3. rank candidates by luma SAD sampled every fourth pixel;
4. evaluate the winner and `(0, 0)` on full-resolution Y, Cb, and Cr;
5. quantize residuals with the current quality step and calculate the best
   fixed-Rice bit count;
6. charge 16 vector bits when the nonzero candidate wins;
7. fall back to co-located prediction unless charged bits strictly improve.

Horizontal vectors are even, so 4:2:2 chroma uses `dx / 2`; vertical
displacement is shared. The model uses source references and is explicitly an
upper-bound potential screen.

## Test

- run all six standard 24-frame videos at q90 and GOP 12;
- retain per-video and per-block rows;
- report baseline/candidate bits, selected blocks, zero/nonzero vectors,
  displacement distribution, SAD search evaluations, and model MP/s;
- prove `(0, 0)` reproduces the baseline calculation;
- use checked indexing and reject all out-of-frame candidates;
- run formatting, release tests, and strict Clippy.

## Gate

- at least 15% charged bit reduction on three videos;
- both natural/camera-derived and synthetic/animation families represented;
- at least 10% of non-key blocks choose a nonzero vector on each advancing
  video;
- model runtime is recorded but does not gate this potential screen.

Passing authorizes a second experiment using reconstructed references, exact
current tile entropy, explicit vector/control bytes, and quality propagation.
It does not authorize format implementation.

## References

- [Research 0029](../research/0029-block-translational-inter-prediction.md)
- [Research 0008](../research/0008-block-local-inter-intra-selection.md)
- [EXP-0005](EXP-0005-gated-temporal-prediction.md)
- [EXP-0047](EXP-0047-compatible-predictor-oracle.md)

## Results

The q90/GOP-12 screen produced 61,380 block rows across all six standard
videos:

| Video | Blocks selecting motion | Charged Rice-bit change | Sampled-SAD change |
|---|---:|---:|---:|
| Blender foliage | 2.18% | -0.22% | -3.36% |
| Blender grass | 23.08% | -4.09% | -21.62% |
| rendered dense motion | 30.80% | -7.59% | -21.96% |
| noisy camera | 11.75% | -1.06% | -9.40% |
| procedural cuts | 55.17% | -17.37% | -65.49% |
| scrolling UI | 30.38% | -11.83% | -38.51% |

Only procedural cuts cleared the 15% rate gate. No natural/camera-derived
video did, and the scrolling UI missed it despite a strong SAD improvement.
This gap is itself evidence that SAD is an imperfect proxy for the folded
residual distribution that Fastvid actually codes.

The most frequent selected vectors were `(16, 8)` on procedural cuts (2,809
blocks), `(0, 4)` on UI (1,075), and `(0, 4)` / `(4, 0)` on rendered dense
motion. The model evaluated 4,557,300 valid block/vector pairs. Throughput was
31.8--52.5 MP/s, far below the speed frontier before exact reconstructed
reference propagation, vector selection, or format work.

Unit tests verified constant-block Rice cost and rejected shifts leaving every
frame edge. The implementation uses checked bounds before its wrapping signed
index conversion. The complete release run passed 52 library tests plus both
motion-model tests. Formatting and warning-clean Clippy passed.

## Artifacts

- per-block model: `artifacts/exp0065-motion-potential.tsv`
  (`ef7874facc9f7c5afb9cbbce140e5ba5c37611bf0f36cfd0f41f5ce3749fe788`);
- per-video summary: `artifacts/exp0065-motion-potential-summary.tsv`
  (`82f960df2f549c09cee1cb8d4a840185ab5759e2986cb14f5be93264d44a5599`);
- analyzer binary:
  `target/release/motion_model`
  (`5f25329e4cdbfe57282aab42ea631125dc477d4cee2f4acc5634a594aff523d3`).

## Decision

**Rejected before exact codec modeling.** The simple integer block-motion
branch is specialized to obvious translations and does not clear the broad
natural-plus-synthetic gate. Do not add motion-vector syntax, search cost, or
new patent surface. Preserve co-located temporal prediction and bounded GOP
access. A future motion revisit would require a materially cheaper,
rate-aware estimator with stronger evidence on natural camera motion, not a
larger search around this failed proxy.
