# EXP-0173 — Full-quality aggressive cell quantizer

Status: **REJECTED**

Date: 2026-07-29

Candidate revision: `400c0aa186d146772f28ed09163b102212557de0`.
Baseline/evaluator revision: `154c51f`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

## Hypothesis

The measured q80 RGB444-10 saving can fund tighter quantization in all cells
that failed the q90 full corpus. One format/depth map will improve rejection
compression materially, clear every full quality failure, and improve total
full-corpus bytes.

## Modification

Change only the unversioned quantization-step mapping:

- gray8 and YUV422-8 use step 1;
- gray10, gray16, and RGB444-16 use denominator 12;
- RGB444-10 uses denominator 5, selecting q90 step 9 instead of 5;
- YUV422-10 and YUV422-16 retain denominators 20 and 12.

Prediction, entropy coding, public API, corpus, evaluator, and bitstream version
remain unchanged. Encoder and decoder use the identical map.

## Canonical commands

```sh
PYTHONPATH=. python3 scripts/evaluate.py --tier rejection \
  --output ARTIFACT.json --libvship-revision v5.0.0 \
  --libvship-build '5.0.0 CUDA direct C API' \
  --libvship-gpu-id 0 --libvship-workers 2

PYTHONPATH=. python3 scripts/evaluate.py --tier full \
  --output /tmp/fastvid-exp0173-cell-quantizer-candidate-full.json \
  --libvship-revision v5.0.0 \
  --libvship-build '5.0.0 CUDA direct C API' \
  --libvship-gpu-id 0 --libvship-workers 2
```

Artifacts:

- baseline rejection:
  `/tmp/fastvid-exp0173-cell-quantizer-baseline-rejection.json`;
- candidate rejection:
  `/tmp/fastvid-exp0173-cell-quantizer-candidate-rejection.json`;
- candidate full:
  `/tmp/fastvid-exp0173-cell-quantizer-candidate-full.json`;
- existing unchanged-map full control:
  `/tmp/fastvid-v7-stratified-corpus-full.json`.

The focused CUDA codec/evaluator suite passed: 26 tests.

## Rejection result

The candidate passed all 11 samples and every correctness, quality, coverage,
and timing gate:

| Mapping | Ratio | Min SSIMULACRA2 | Max Butteraugli |
|---|---:|---:|---:|
| baseline | 6.188000859x | 93.697319 | 0.084405 |
| candidate | 6.980224590x | 92.772705 | 0.147904 |

This is a 12.8% ratio improvement and qualified the unchanged candidate for
the full tier.

## Full result

The 397-sample full run completed in 115.285 seconds but failed:

- `xiph-sintel-01000-rgb444-10`: SSIMULACRA2 85.989182;
- `game-minetest-rgb444-10`: SSIMULACRA2 89.744324;
- `ai-14-rgb444-16`: encode 1.012816 ms and decode 0.547984 ms, failing both
  single-frame latency gates.

Maximum Butteraugli was 0.367659 and was not the limiting quality metric.
The candidate encoded 2,228,261,500 bytes at 6.141900311x. The unchanged q90
full control encoded 2,145,624,455 bytes at 6.378450790x, so the candidate
expanded the full corpus by 82,637,045 bytes or 3.851%. It repaired 19 of the
21 quality failures exposed by EXP-0168, but introduced the two RGB10 failures.

## Conclusion

