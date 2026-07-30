# EXP-0182 — Median edge predictor

Status: **REJECTED**

Date: 2026-07-30

Candidate revision: `cc28c50d3317dfe097d0d800e108f1e817215fb2`.
Baseline revision: `df9cd21f7bcc4aefbcda1fad64e87421723358fa`.
Evaluator revision: `ed4febd5b9b2815fc86e1f36e50ef4a63b8eac46`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

## Hypothesis and change

Replacing the unconstrained clamped-gradient predictor with the JPEG-LS median
edge detector (MED) will eliminate edge overshoot, reduce residual entropy,
and stabilize causal reconstruction without changing quantizer steps or adding
metadata. Encoder and both CUDA decoder paths use the identical predictor.

## Canonical command and artifacts

```sh
PYTHONPATH=. python3 scripts/evaluate.py --tier rejection \
  --output /tmp/fastvid-exp0182-med-predictor-candidate-rejection.json \
  --libvship-revision v5.0.0 \
  --libvship-build '5.0.0 CUDA direct C API' \
  --libvship-gpu-id 0 --libvship-workers 2
```

- baseline: `/tmp/fastvid-edit-resilience-baseline-rejection.json`;
- candidate: `/tmp/fastvid-exp0182-med-predictor-candidate-rejection.json`.

The focused evaluator/API suite passed 16 tests.

## Result

| Codec | Bytes | Ratio | Min SSIMU2 | Max Butter | Generation min/max |
|---|---:|---:|---:|---:|---:|
| baseline | 347,833,953 | 6.188001x | 93.697319 | 0.803438 | 87.446571 / 2.702818 |
| candidate | 319,556,845 | 6.735568x | 93.697289 | 0.803438 | 87.446571 / 2.702818 |

The candidate saved 28,277,108 bytes (8.13%) and passed ordinary perceptual,
correctness, coverage, and every timing gate. It reduced generation failures
from nine to five, but YUV8, YUV10, YUV16, RGB10, and gray10 still failed.
Full was not run.

## Conclusion

Reject on generation robustness, but MED is the strongest rate result in this
sequence and creates an 8.13% repair budget. A clean successor should combine
MED with context-independent sample-lattice quantization before spending that
budget on narrower quality repair.

Related: [research 0041](../research/0041-adaptive-med-block-predictor.md),
[EXP-0178](EXP-0178-absolute-sample-lattice.md), and
[research 0049](../research/0049-multi-generation-quantization-drift.md).
