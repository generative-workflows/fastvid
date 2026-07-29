# EXP-0174 — Corrected-Butteraugli cell quantizer

Status: **REJECTED**

Date: 2026-07-29

Candidate revision: `c2f6129a3dcd687c2ba8b3f88d951c2eea1d7ffd`.
Baseline/evaluator revision: `dc8d2e5301aa0a9e79b21ee750d27d09b072deab`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

## Hypothesis and modification

Repeating EXP-0173 with corrected 80-nit libjxl-compatible Butteraugli will
show whether its aggressive format/depth quantizer map still passes rejection.
Exactly its map was reapplied: gray8 and YUV422-8 step 1; gray10, gray16, and
RGB444-16 denominator 12; RGB444-10 denominator 5; YUV422-10/16 unchanged.
Predictor, entropy coder, API, corpus, and evaluator are unchanged.

## Canonical command and artifacts

```sh
PYTHONPATH=. python3 scripts/evaluate.py --tier rejection \
  --output /tmp/fastvid-exp0174-fixed-butteraugli-cell-map-candidate-rejection.json \
  --libvship-revision v5.0.0 \
  --libvship-build '5.0.0 CUDA direct C API' \
  --libvship-gpu-id 0 --libvship-workers 2
```

- baseline: `/tmp/fastvid-fixed-butteraugli-baseline-rejection.json`;
- candidate: `/tmp/fastvid-exp0174-fixed-butteraugli-cell-map-candidate-rejection.json`;
- corrected baseline full: `/tmp/fastvid-fixed-butteraugli-baseline-full.json`.

## Result

| Mapping | Encoded bytes | Ratio | Min SSIMULACRA2 | Max Butteraugli |
|---|---:|---:|---:|---:|
| baseline | 347,833,953 | 6.188001x | 93.697319 | 0.803438 |
| candidate | 308,356,382 | 6.980225x | 92.772705 | 1.123859 |

The candidate saved 39,477,571 bytes (11.35%), but frame 5 of
`performance-4k-x24-rgb444-10` failed the Butteraugli gate of 1.0. Correctness
and timing passed. Per immediate rejection, full was not run.

## Conclusion

Reject. EXP-0173's quality pass depended on broken one-nit Butteraugli. The
RGB10 rate gain is real, but needs adaptive protection of local distortion.

Related: [research 0048](../research/0048-corrected-butteraugli-frontier.md) and
[EXP-0173](EXP-0173-full-quality-aggressive-cell-quantizer.md).
