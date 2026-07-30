# EXP-0177 — Three-zone tile precision

Status: **REJECTED**

Date: 2026-07-30

Candidate revision: `6ccd7a7b5d107f691d4d41d29cc0407a57837210`.
Baseline revision: `df9cd21f7bcc4aefbcda1fad64e87421723358fa`.
Evaluator revision: `ed4febd5b9b2815fc86e1f36e50ef4a63b8eac46`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

## Hypothesis and change

A normalized mean-gradient proxy can allocate the repair steps measured in
EXP-0176 only where repeated edits are vulnerable, while coarser steps in
strongly textured tiles fund the cost. Per tile, mean horizontal/vertical
gradient below 2 eight-bit code values selects the format-specific repair
step, 2 through 6 retains the baseline step, and at least 6 selects roughly
twice the baseline step. The class is signaled in the tile directory.

## Canonical command and artifacts

```sh
PYTHONPATH=. python3 scripts/evaluate.py --tier rejection \
  --output /tmp/fastvid-exp0177-three-zone-tile-candidate-rejection.json \
  --libvship-revision v5.0.0 \
  --libvship-build '5.0.0 CUDA direct C API' \
  --libvship-gpu-id 0 --libvship-workers 2
```

- baseline: `/tmp/fastvid-edit-resilience-baseline-rejection.json`;
- candidate: `/tmp/fastvid-exp0177-three-zone-tile-candidate-rejection.json`.

The focused evaluator/API suite passed 16 tests.

## Result

| Codec | Bytes | Ratio | Min SSIMU2 | Max Butter | Generation min/max |
|---|---:|---:|---:|---:|---:|
| baseline | 347,833,953 | 6.188001x | 93.697319 | 0.803438 | 87.446571 / 2.702818 |
| candidate | 408,527,779 | 5.268667x | 90.018837 | 1.618436 | 77.310150 / 4.041321 |

The candidate expanded rejection output by 60,693,826 bytes (17.45%). Nine
samples failed generation robustness, `ai-13-gray-10` also failed ordinary
perceptual quality, and maximum generation Butteraugli worsened to 4.041321.
Correctness, coverage, and all timing gates passed. Full was not run.

## Conclusion

Reject. Tile mean gradient is not a sufficient masking or edit-resilience
proxy. High-gradient quantization created worse local and accumulated errors,
while the low-gradient repair region did not cover the vulnerable regions.
Do not tune these thresholds further without a different sensitivity signal.

Related: [EXP-0175](EXP-0175-activity-adaptive-tile-quantizer.md),
[EXP-0176](EXP-0176-edit-resilient-quantizer-map.md), and
[research 0048](../research/0048-corrected-butteraugli-frontier.md).
