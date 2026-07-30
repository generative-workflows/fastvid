# EXP-0178 — Absolute sample-lattice quantization

Status: **REJECTED**

Date: 2026-07-30

Candidate revision: `41356386741e18ed1fdd04ce57a49d077e641015`.
Baseline revision: `df9cd21f7bcc4aefbcda1fad64e87421723358fa`.
Evaluator revision: `ed4febd5b9b2815fc86e1f36e50ef4a63b8eac46`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

## Hypothesis and change

Quantizing source samples on an absolute lattice before prediction will remove
causal predictor-dependent quantization drift, improve ten-cycle edit
resilience, and preserve first-pass quality and rate. The existing gradient
predictor losslessly decorrelates quantized integer indices; the decoder then
scales indices to samples in one CUDA kernel. Quantizer steps, entropy coding,
corpus, and evaluator are unchanged.

## Canonical command and artifacts

```sh
PYTHONPATH=. python3 scripts/evaluate.py --tier rejection \
  --output /tmp/fastvid-exp0178-absolute-lattice-candidate-rejection.json \
  --libvship-revision v5.0.0 \
  --libvship-build '5.0.0 CUDA direct C API' \
  --libvship-gpu-id 0 --libvship-workers 2
```

- baseline: `/tmp/fastvid-edit-resilience-baseline-rejection.json`;
- candidate: `/tmp/fastvid-exp0178-absolute-lattice-candidate-rejection.json`.

The focused evaluator/API suite passed 16 tests.

## Result

| Codec | Bytes | Ratio | Min SSIMU2 | Max Butter | Generation min/max |
|---|---:|---:|---:|---:|---:|
| baseline | 347,833,953 | 6.188001x | 93.697319 | 0.803438 | 87.446571 / 2.702818 |
| candidate | 347,812,453 | 6.188383x | 93.697289 | 0.841698 | 86.627052 / 2.987740 |

The candidate saved 21,500 bytes (0.0062%) and passed ordinary perceptual,
correctness, coverage, and all timing gates. Generation failures fell from
nine to five: YUV10, YUV16, YUV8, RGB10, and gray10 remained. Full was not run
because rejection did not pass.

## Conclusion

Reject on generation robustness, but retain the mechanism as the strongest
measured follow-up foundation. It simultaneously improved rate and repaired
four generation cases without lower quantizer steps. A clean successor should
combine this absolute lattice with targeted refinement only in the five
remaining failing format/depth cells.

Related research: [0049](../research/0049-multi-generation-quantization-drift.md).
