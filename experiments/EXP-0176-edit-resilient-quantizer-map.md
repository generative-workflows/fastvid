# EXP-0176 — Edit-resilient quantizer map

Status: **REJECTED**

Date: 2026-07-29

Candidate revision: `ae24d38b89d4ef7b83629356f2e322c716c99d86`.
Baseline codec revision: `df9cd21f7bcc4aefbcda1fad64e87421723358fa`.
Evaluator revision: `ed4febd5b9b2815fc86e1f36e50ef4a63b8eac46`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

## Hypothesis and change

A single finer format/depth quantizer map inferred from EXP-0175's repaired
cells will pass the ten-cycle edit-resilience gate: q90 steps are YUV8 1,
YUV10 2, YUV16 108, RGB10 3, RGB16 215, gray10 3, and gray16 215. Only the
encoder/decoder quantization-step mapping changed.

## Canonical command and artifacts

```sh
PYTHONPATH=. python3 scripts/evaluate.py --tier rejection \
  --output /tmp/fastvid-exp0176-edit-resilient-quantizer-candidate-rejection.json \
  --libvship-revision v5.0.0 \
  --libvship-build '5.0.0 CUDA direct C API' \
  --libvship-gpu-id 0 --libvship-workers 2
```

- baseline: `/tmp/fastvid-edit-resilience-baseline-rejection.json`;
- candidate: `/tmp/fastvid-exp0176-edit-resilient-quantizer-candidate-rejection.json`.

The focused evaluator/API suite passed 16 tests.

## Result

| Codec | Bytes | Ratio | Min SSIMU2 | Max Butter | Generation min/max |
|---|---:|---:|---:|---:|---:|
| baseline | 347,833,953 | 6.188001x | 93.697319 | 0.803438 | 87.446571 / 2.702818 |
| candidate | 444,115,469 | 4.846480x | 97.928894 | 0.295470 | 94.251152 / 1.271740 |

The candidate expanded rejection output by 96,281,516 bytes (27.68%). Three
generation cases remained above Butteraugli 1.0: YUV10 `1.092684`, YUV16
`1.271740`, and gray10 `1.067237`. Both RGB10 latency samples exceeded the
1.0 ms encode and 0.5 ms decode limits. Correctness and coverage passed.
Full was not run.

## Conclusion

Reject on generation quality, timing, and compression. The map repaired six
of the baseline's nine generation failures, including YUV8, gray16, RGB10,
and RGB16, but global refinement costs too many bytes and still does not clear
YUV10/YUV16/gray10. A viable successor needs spatial precision allocation and
an entropy or prediction gain large enough to fund it.

Related: [EXP-0175](EXP-0175-activity-adaptive-tile-quantizer.md) and
[research 0048](../research/0048-corrected-butteraugli-frontier.md).
