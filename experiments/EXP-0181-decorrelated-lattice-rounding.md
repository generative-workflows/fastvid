# EXP-0181 — Decorrelated lattice rounding

Status: **REJECTED**

Date: 2026-07-30

Candidate revision: `6a10050ee450a0ef0c75beb0bb23b2f9b606fc26`.
Baseline revision: `df9cd21f7bcc4aefbcda1fad64e87421723358fa`.
Evaluator revision: `ed4febd5b9b2815fc86e1f36e50ef4a63b8eac46`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

## Hypothesis and change

Replacing nearest rounding on EXP-0178's absolute sample lattice with a
deterministic 8x8 permutation of thresholds will decorrelate quantization error
across spatial edits and reduce ten-cycle accumulation without changing steps
or signaling side information. Prediction still losslessly decorrelates the
quantized indices; only the rounding threshold varies by absolute coordinate.

## Canonical command and artifacts

```sh
PYTHONPATH=. python3 scripts/evaluate.py --tier rejection \
  --output /tmp/fastvid-exp0181-decorrelated-lattice-candidate-rejection.json \
  --libvship-revision v5.0.0 \
  --libvship-build '5.0.0 CUDA direct C API' \
  --libvship-gpu-id 0 --libvship-workers 2
```

- baseline: `/tmp/fastvid-edit-resilience-baseline-rejection.json`;
- candidate: `/tmp/fastvid-exp0181-decorrelated-lattice-candidate-rejection.json`.

The focused evaluator/API suite passed 16 tests.

## Result

| Codec | Bytes | Ratio | Min SSIMU2 | Max Butter | Generation min/max |
|---|---:|---:|---:|---:|---:|
| baseline | 347,833,953 | 6.188001x | 93.697319 | 0.803438 | 87.446571 / 2.702818 |
| candidate | 362,843,354 | 5.932028x | 93.103462 | 0.853365 | 86.469673 / 2.161211 |

The candidate expanded output by 15,009,401 bytes (4.32%). It passed ordinary
perceptual, correctness, coverage, and timing gates, but five generation cases
failed. YUV16 was repaired relative to EXP-0178 while RGB16 became a new
failure; YUV8, YUV10, RGB10, and gray10 remained. Full was not run.

## Conclusion

Reject on generation quality and compression. Spatial threshold variation
redistributes error but does not reduce the failure count, and its less
concentrated symbol distribution costs material rate. Do not continue with
ordered/stochastic rounding at unchanged steps.

Related: [EXP-0178](EXP-0178-absolute-sample-lattice.md) and
[research 0049](../research/0049-multi-generation-quantization-drift.md).
