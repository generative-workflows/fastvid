# EXP-0175 — Activity-adaptive tile quantizer

Status: **SUPERSEDED**

Date: 2026-07-29

Candidate revision: `49ab33734d1e0e1fef9fcbbe417746214366c8d7`.
Restored codec revision: `df9cd21f7bcc4aefbcda1fad64e87421723358fa`.
Evaluator revision: `ed4febd5b9b2815fc86e1f36e50ef4a63b8eac46`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

## Hypothesis and change

Lossless YUV8 and finer gray/RGB16 tile classes will repair known vulnerable
cells, while RGB10 step 9 on tiles whose mean horizontal/vertical gradient is
at least 8 code values will retain rate gains without local Butteraugli peaks.
The existing directory byte signaled four quantizer classes independently per
tile. Prediction, entropy coding, corpus, and evaluator were unchanged.

## Canonical command and artifacts

```sh
PYTHONPATH=. python3 scripts/evaluate.py --tier rejection \
  --output /tmp/fastvid-exp0175-adaptive-tile-quantizer-candidate-rejection.json \
  --libvship-revision v5.0.0 \
  --libvship-build '5.0.0 CUDA direct C API' \
  --libvship-gpu-id 0 --libvship-workers 2
```

- current-evaluator baseline, measured after the candidate:
  `/tmp/fastvid-edit-resilience-baseline-rejection.json`;
- candidate:
  `/tmp/fastvid-exp0175-adaptive-tile-quantizer-candidate-rejection.json`.

## Result

| Codec | Bytes | Ratio | Min SSIMU2 | Max Butter | Generation min/max |
|---|---:|---:|---:|---:|---:|
| baseline | 347,833,953 | 6.188001x | 93.697319 | 0.803438 | 87.446571 / 2.702818 |
| candidate | 335,203,722 | 6.421160x | 93.697319 | 1.123859 | 88.281296 / 2.702818 |

The candidate saved 12,630,231 bytes (3.63%) but failed ordinary RGB10
Butteraugli and seven generation-robustness cases. The restored baseline fails
nine generation cases and two timing gates. The candidate passed correctness
and its measured timing gates. Full was not run.

## Conclusion

Supersede because evaluator commit `ed4febd` landed after the earlier baseline
artifact and the matching baseline was consequently measured after the
candidate, violating the prescribed experiment order. Independently, the
candidate fails absolute rejection gates and cannot be accepted. Its repaired
YUV8, gray16, and RGB16 generation cases motivate a clean follow-up using the
new baseline; mean-gradient threshold 8 does not protect RGB10 local quality.

Related research: [0048](../research/0048-corrected-butteraugli-frontier.md).
