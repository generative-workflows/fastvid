# EXP-0198 — Offset absolute gray10 lattice

Status: **REJECTED**

Date: 2026-07-30

Baseline revision: `f5f4b9e` (codec source from accepted `1f8ff8c`).
Baseline codec-source SHA-256: `c249ce4bc02dee2eab5f2972ce80b764ac1fd22e634059f74bb6f76ac6e5f1ce`.
Evaluator revision: `ed4febd5b9b2815fc86e1f36e50ef4a63b8eac46`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

## Hypothesis and rationale

For lossy gray10, quantize samples onto fixed bin centers `step/2 + k*step`
before spatial prediction, rather than quantizing predictor-relative residuals.
At q90 this is the offset lattice `4 + 8k`. It will remove predictor-dependent
lattice drift, strictly improve controlling `ai-13-gray-10` generation quality,
introduce no new failure/regression, and not increase bytes.

EXP-0196 and EXP-0197 establish accepted step 8 as a local optimum versus
steps 7 and 9, so scalar tuning is exhausted. The canonical ai-13 source has
uniform modulo-8 residues (each class 12.45–12.55%), giving no content-specific
reason to prefer the prior zero-offset absolute lattice. A half-step-centered
lattice is the complementary fixed-point geometry. The encoder predicts exact
lattice indices; the decoder reconstructs indices and applies one gray10-only
scaling kernel. Every other format/depth and q100 remain unchanged.

## Canonical command and artifacts

```sh
PYTHONPATH=.:cuda python3 scripts/evaluate.py --tier <rejection|full> \
  --output <artifact> --libvship-revision v5.0.0 \
  --libvship-build '5.0.0 CUDA direct C API' \
  --libvship-gpu-id 0 --libvship-workers 2
```

- rejection baseline cache hit: `evaluation_results/rejection-c249ce4bc02dee2eab5f2972ce80b764ac1fd22e634059f74bb6f76ac6e5f1ce.json`;
- full baseline cache hit if required: `evaluation_results/full-c249ce4bc02dee2eab5f2972ce80b764ac1fd22e634059f74bb6f76ac6e5f1ce.json`.

Candidate codec-source SHA-256:
`9cf7013027600e5627dfd28860ef3d10a31326e0df3b9786f00b33577d36474c`.
Candidate patch ID: `e44bdf4793a49b0cd30264ce6a7f022f89f9e894`.

- rejection candidate: `evaluation_results/rejection-9cf7013027600e5627dfd28860ef3d10a31326e0df3b9786f00b33577d36474c.json`
  (artifact SHA-256 `6379fc3f241099188009d8314834350516e7b973ab45ab7a064f8eaf447fc01e`).

## Result

The candidate encoded 322,212,344 bytes versus 322,248,140 for the baseline,
saving 35,796 bytes (0.011%) and improving ratio from 6.679315x to 6.680057x.
Correctness, determinism, coverage, and all performance gates passed. The
extra gray10 scaling kernel increased `ai-13` decode from 0.375008 to
0.432256 ms but remained under its gate.

Quality regressed. Ordinary maximum Butteraugli rose from 0.747622 to 0.851072
while minimum SSIMULACRA2 remained above the global floor. Generation minimum
SSIMULACRA2 fell from 89.081276 to 86.143089 and maximum Butteraugli rose from
2.482678 to 3.030743, all controlled by `ai-13-gray-10`. Failure identities
and count remained unchanged at five.

## Conclusion

Rejected at the rejection tier because both worst generation violations became
strictly worse. Together with EXP-0178's zero-offset result, this rejects both
natural modulo-step placements for a global absolute gray10 lattice. No full
evaluation is permitted. Source changes were reverted after recording the
result; the next branch should alter prediction/context rather than scalar or
lattice geometry.

Related: [EXP-0178](EXP-0178-absolute-sample-lattice.md),
[EXP-0196](EXP-0196-gray10-step7-rgb16-funding.md),
[EXP-0197](EXP-0197-gray10-step9-stability.md), and
[research 0049](../research/0049-multi-generation-quantization-drift.md).
