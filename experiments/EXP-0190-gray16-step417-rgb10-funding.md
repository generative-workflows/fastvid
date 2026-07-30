# EXP-0190 — Gray16 step 417 with RGB10 entropy funding

Status: **REJECTED**

Date: 2026-07-30

Baseline revision: `b9b63da` (codec source unchanged from `3ae880c`).
Baseline codec-source SHA-256: `b86e33fecb2f0e7d317f7f621acacaecae618f34f9ade22e739ae9460567680f`.
Evaluator revision: `ed4febd5b9b2815fc86e1f36e50ef4a63b8eac46`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

## Hypothesis and rationale

At q90, changing gray16's reconstruction step from 428 to 417 will strictly
reduce the baseline's controlling `procedural-02-gray-16` quality violation
without crossing `raw-05-gray-16`'s generation Butteraugli gate. Enabling the
existing exact entropy competition only for 4K RGB444-10 will fund the modest
gray16 rate cost, reduce rejection and full bytes, and avoid EXP-0188's RGB16
latency regression. The candidate will introduce no new failure or regression.

The step is a falsifiable probe derived from EXP-0188's measured `raw-05`
generation Butteraugli values: 0.989364 at step 428 and 1.085619 at step 321
linearly cross 1.0 near 416.2. Rounding upward gives 417. The general depth-
scaled gray16 formula uses rational denominator 80/13, producing step 417 at
q90 and step 1 at q100. All syntax, predictors, decoder entropy paths, and
other format/depth quantizers remain unchanged.

## Canonical command and artifacts

```sh
PYTHONPATH=.:cuda python3 scripts/evaluate.py --tier <rejection|full> \
  --output <artifact> --libvship-revision v5.0.0 \
  --libvship-build '5.0.0 CUDA direct C API' \
  --libvship-gpu-id 0 --libvship-workers 2
```

- rejection baseline cache hit: `evaluation_results/rejection-b86e33fecb2f0e7d317f7f621acacaecae618f34f9ade22e739ae9460567680f.json`;
- full baseline cache hit if required: `evaluation_results/full-b86e33fecb2f0e7d317f7f621acacaecae618f34f9ade22e739ae9460567680f.json`.

Candidate codec-source SHA-256:
`2f0b66c17c7a5570306c64454e3dbe5021a38f994aa982b72e094e3a3f3c331b`.
Candidate patch ID: `6a8a70cf4908db8f695f2d462d88f9fb5dbeb3da`.

- rejection candidate: `evaluation_results/rejection-2f0b66c17c7a5570306c64454e3dbe5021a38f994aa982b72e094e3a3f3c331b.json`
  (artifact SHA-256 `49e61c68739447c935256fb63e6b8f46a144318a62517cb2ea41ec5e3c19f838`);
- full candidate: `evaluation_results/full-2f0b66c17c7a5570306c64454e3dbe5021a38f994aa982b72e094e3a3f3c331b.json`
  (artifact SHA-256 `36f000af5aac4445c0dbd322cacd09aca60c3469062ea225b6e8b4b568a25b26`).

## Results

The rejection candidate encoded 321,819,938 bytes versus 323,186,668 for the
baseline, saving 1,366,730 bytes (0.423%) and improving ratio from 6.659918x to
6.688202x. Ordinary extrema remained 94.813339 / 0.747622. The generation
SSIMULACRA2 floor improved from 88.169777 to 88.462814 while maximum
Butteraugli remained 2.482678. Both artifacts had the same five quality
failures and no correctness, determinism, coverage, or performance failure.

The full candidate encoded 2,123,867,728 bytes versus 2,130,251,655 for the
baseline: **6,383,927 fewer bytes** (0.300%), improving ratio from 6.424480x to
6.443791x. Ordinary extrema improved from 86.211967 / 1.632229 to
86.811913 / 1.632229. Generation extrema improved from 81.113586 / 4.645103 to
81.678612 / 4.645103.

However, failures increased from 174 to 176. The candidate resolved
`meridian-01000-gray-16`'s ordinary-quality failure but introduced:

- `game-veloren-gray-16: perceptual quality gate failed`;
- `raw-26-gray-16: generation robustness quality gate failed`;
- `performance-1080p-rgb444-10: decode latency >= 0.5 ms`.

The controlling 1080p RGB10 decode median regressed from 0.477984 ms to
0.560416 ms. Candidate 4Kx24 YUV10 encode/decode medians were 58.120/27.713 ms;
4Kx24 RGB10 medians were 84.261/39.756 ms. Correctness and determinism passed.

## Conclusion

Rejected. Despite a useful full-corpus size reduction and strict improvement
to the worst SSIMULACRA2 violation, two new quality failures violate the
no-new-failures rule and the 1080p decode latency failure independently mandates
rejection. The step-417 interpolation did keep `raw-05-gray-16` passing, but
generation quality is non-monotone across other content (`raw-26`) and cannot
be repaired safely with a single global gray16 step. Source changes were
reverted after recording the result.

Related: [EXP-0188](EXP-0188-4k-rgb-entropy-funded-gray16.md),
[EXP-0189](EXP-0189-aligned-gray16-rgb10-funding.md), and
[research 0050](../research/0050-jpeg-xs-wavelet-perceptual-allocation.md).
