# EXP-0180 — Lossless entropy competition

Status: **REJECTED**

Date: 2026-07-30

Candidate revision: `f519395c8569af38ac670828f4fd9fac9f1a9cbb`.
Baseline revision: `df9cd21f7bcc4aefbcda1fad64e87421723358fa`.
Evaluator revision: `ed4febd5b9b2815fc86e1f36e50ef4a63b8eac46`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

## Hypothesis and change

Lossless q90 reconstruction will clear ordinary and ten-cycle perceptual gates,
while allowing RGB shards to choose the smallest complete-byte result among
order-0 rANS, Rice, zero-run, and block packing will improve the q100 boundary
enough to test speed feasibility. Quantizer step is 1 for every required cell;
only RGB's forced order-0 policy is removed.

## Canonical command and artifacts

```sh
PYTHONPATH=. python3 scripts/evaluate.py --tier rejection \
  --output /tmp/fastvid-exp0180-lossless-entropy-competition-candidate-rejection.json \
  --libvship-revision v5.0.0 \
  --libvship-build '5.0.0 CUDA direct C API' \
  --libvship-gpu-id 0 --libvship-workers 2
```

- q90 baseline: `/tmp/fastvid-edit-resilience-baseline-rejection.json`;
- q100 forced-order0 diagnostic: `/tmp/fastvid-edit-resilience-q100-rejection.json`;
- candidate: `/tmp/fastvid-exp0180-lossless-entropy-competition-candidate-rejection.json`.

The focused evaluator/API suite passed 16 tests.

## Result

| Codec | Bytes | Ratio | Min SSIMU2 | Max Butter | Generation min/max |
|---|---:|---:|---:|---:|---:|
| q90 baseline | 347,833,953 | 6.188001x | 93.697319 | 0.803438 | 87.446571 / 2.702818 |
| lossless candidate | 590,442,819 | 3.645394x | 99.601753 | 0.0 | 99.593117 / 0.0 |

The candidate is exact by Butteraugli through ten edit cycles and improves the
forced-order0 q100 ratio from 3.575715x to 3.645394x. Against the required q90
baseline it expands by 242,608,866 bytes (69.75%). RGB10 encode/decode measured
1.040160/0.557904 ms and 1.041808/0.555648 ms on the two latency samples;
RGB16 decode measured 0.613024 ms. All five exceed their gates. Correctness and
coverage passed. Full was not run.

## Conclusion

Reject on compression and timing. Complete entropy competition recovers about
1.95% versus forced order-0 lossless coding, but a valid candidate still needs
to remove 41.09% of lossless bytes merely to match the q90 baseline and must
accelerate RGB decode. This establishes the lossless feasibility boundary;
further progress needs a materially better reversible predictor/transform or
an error-stable near-lossless reconstruction, not another entropy selector.

Related research: [0049](../research/0049-multi-generation-quantization-drift.md).
