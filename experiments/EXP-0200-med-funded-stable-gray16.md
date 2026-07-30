# EXP-0200 — MED-funded stable gray16 refinement

Status: **ACCEPTED**

Date: 2026-07-30

Baseline revision: `055c002` (codec source retained from accepted EXP-0193).
Baseline codec-source SHA-256:
`c249ce4bc02dee2eab5f2972ce80b764ac1fd22e634059f74bb6f76ac6e5f1ce`.
Evaluator revision: `ed4febd5b9b2815fc86e1f36e50ef4a63b8eac46`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

## Hypothesis and rationale

EXP-0194 predicted that widening the accepted gray16 selector's low-rate arm
from 0.10 to 0.20 byte/sample keeps edited `procedural-02-gray-16` on step 321
through all generations and repairs the full-corpus generation SSIMULACRA2
controller, but that change was invisible in rejection and could not advance.
EXP-0199 independently established that selecting MED for lossy gray10 above
0.30 baseline payload byte/sample improves rejection's `ai-13-gray-10`
controller and saves 2,368,062 full-tier bytes without new failures. Its full
result was rejected only because it did not touch the global full controller.

The candidate composes those complementary rate and quality arms into one
deterministic allocation policy:

- lossy gray10 above 0.30 baseline payload byte/sample uses MED and signals
  bit 6 of the depth byte;
- gray16 below 0.20 or above 0.50 baseline payload byte/sample uses the
  accepted step-321 refinement and signals bit 7.

Nongray prediction, gray steps outside those regions, entropy coding, tile
geometry, and decoder scheduling are unchanged. The falsifiable hypothesis is
that the MED arm will reproduce EXP-0199's strict rejection improvement and
rate saving, while the widened gray16 arm will strictly improve the actual
full-corpus worst generation violation. Their measured budgets predict no
size increase, no new failures, and all non-quality gates passing.

## Canonical command and artifacts

```sh
PYTHONPATH=.:cuda python3 scripts/evaluate.py --tier <rejection|full> \
  --output <source-keyed-artifact> --libvship-revision v5.0.0 \
  --libvship-build '5.0.0 CUDA direct C API' \
  --libvship-gpu-id 0 --libvship-workers 2
```

- rejection baseline cache hit:
  `evaluation_results/rejection-c249ce4bc02dee2eab5f2972ce80b764ac1fd22e634059f74bb6f76ac6e5f1ce.json`;
- full baseline cache hit if required:
  `evaluation_results/full-c249ce4bc02dee2eab5f2972ce80b764ac1fd22e634059f74bb6f76ac6e5f1ce.json`.

Candidate codec-source SHA-256:
`a694cd12c51b445edb6f6e33e5f2b7f4a0611aa23d53e66301a59ee150d78b74`.

- candidate rejection:
  `evaluation_results/rejection-a694cd12c51b445edb6f6e33e5f2b7f4a0611aa23d53e66301a59ee150d78b74.json`.

## Result

The focused evaluator/API/CUDA suite passed all 35 tests.

| Codec | Bytes | Ratio | Ordinary min/max | Generation min/max | Failures |
|---|---:|---:|---:|---:|---:|
| baseline | 322,248,140 | 6.679315x | 94.813339 / 0.747622 | 89.081276 / 2.482678 | 5 |
| candidate | 322,199,847 | 6.680316x | 94.813339 / 0.747542 | 89.327042 / 2.308000 | 5 |

The rejection result exactly reproduces EXP-0199 because the widened gray16
arm selects no additional rejection frame. It saves 48,293 bytes, strictly
improves both controlling generation extrema, and introduces no failure or
non-quality regression. Rejection artifact SHA-256 is
`b1b4a3b9b6ba3f84a97e05375f3d560540344ff56de1b380f85ab47df7c9bc6a`.

The unchanged source advanced to full:

- candidate full:
  `evaluation_results/full-a694cd12c51b445edb6f6e33e5f2b7f4a0611aa23d53e66301a59ee150d78b74.json`.

| Codec | Bytes | Ratio | Ordinary min/max | Generation min/max | Failures |
|---|---:|---:|---:|---:|---:|
| baseline | 2,123,884,240 | 6.443741x | 88.391052 / 1.632229 | 81.743011 / 4.645103 | 173 |
| candidate | 2,122,552,061 | 6.447785x | 88.391052 / 1.632229 | 85.083862 / 4.645103 | 173 |

The candidate saves 1,332,179 bytes (0.0627%), strictly improves the actual
full-corpus worst generation SSIMULACRA2 by 3.340851, introduces no new failure
identity, and passes every correctness, coverage, determinism, and performance
gate. `procedural-02-gray-16` improves from 81.743011 / 4.433615 to
87.259956 / 3.196179 after ten generations. The known selector hazards remain
isolated: `raw-40-gray-10` and `raw-05-gray-16` are byte-for-byte and
metric-for-metric unchanged. Full artifact SHA-256 is
`8388a3ae17f94890de1a2e873d782b89557d6c0e19c9c1367f5b107d4c2ebe69`.

## Conclusion

Accept under the failing-baseline exception. The gray10 MED arm supplies both
the rejection-tier quality movement and enough full-corpus rate budget for the
wider gray16 stability region. The gray16 arm then repairs the true full-tier
controller, which neither component could establish alone. Retain both
source-visible flags, the 0.30-byte/sample gray10 MED gate, and the widened
0.20/0.50-byte/sample gray16 selector as the new pre-v1 baseline.

Related: [EXP-0194](EXP-0194-stable-gray16-rgb16-funding.md),
[EXP-0199](EXP-0199-high-rate-gray10-med.md), and
[research 0049](../research/0049-multi-generation-quantization-drift.md).
