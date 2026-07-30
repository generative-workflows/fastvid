# EXP-0183 — MED prediction on an absolute sample lattice

Status: **REJECTED**

Date: 2026-07-30

Baseline revision: `2fc357b69e1264f8c7a32c51406b2967897ffe2e`.
Candidate source patch ID: `1f35b35e667304fdfe6a42fa361a695768a64d37`.
Evaluator revision: `ed4febd5b9b2815fc86e1f36e50ef4a63b8eac46`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

## Hypothesis and change

Combining JPEG-LS median-edge prediction with absolute sample-lattice
quantization will retain MED's measured 8.13% compression gain while removing
predictor-dependent quantization drift. Relative to the failing baseline, the
candidate will strictly reduce the worst generation-quality violation,
introduce no new failures or regressions, and not increase compressed size.

The one attributable change is the reconstruction rule: quantize source
samples to a fixed integer lattice, predict lattice indices with MED, entropy
code their residual, reconstruct indices identically in both decoder paths,
then scale once to native samples. Entropy modes, quantizer steps, metadata,
corpus, and evaluator are unchanged.

## Canonical command and artifacts

```sh
PYTHONPATH=.:cuda python3 scripts/evaluate.py --tier rejection \
  --output /tmp/fastvid-exp0183-{baseline,candidate}-rejection.json \
  --libvship-revision v5.0.0 \
  --libvship-build '5.0.0 CUDA direct C API' \
  --libvship-gpu-id 0 --libvship-workers 2
```

- baseline: `/tmp/fastvid-exp0183-baseline-rejection.json`;
- candidate: `/tmp/fastvid-exp0183-candidate-rejection.json`.

The authoritative baseline extension was freshly rebuilt from restored source.
A preceding stale-extension run was overwritten and is not evidence.

## Baseline

The rejection baseline encoded 347,833,953 bytes at 6.188001x, with minimum
SSIMULACRA2 93.697319 and maximum Butteraugli 0.803438. Ordinary quality passed,
but generation robustness failed nine cases.

## Result

The focused evaluator/API suite passed all 35 tests.

| Codec | Bytes | Ratio | Min SSIMU2 | Max Butter | Generation min/max | Failures |
|---|---:|---:|---:|---:|---:|---:|
| baseline | 347,833,953 | 6.188001x | 93.697319 | 0.803438 | 87.446571 / 2.702818 | 9 |
| candidate | 319,545,160 | 6.735814x | 93.697289 | 0.841698 | 86.627052 / 2.987740 | 5 |

The candidate saved 28,288,793 bytes (8.13%) and removed four generation
failures. It nevertheless worsened both worst generation extrema: minimum
SSIMULACRA2 fell by 0.819519 and maximum Butteraugli rose by 0.284922. The full
tier was not run because the rejection comparison cannot satisfy the failing-
baseline acceptance exception.

## Conclusion

Reject and restore the baseline codec. MED and absolute-lattice quantization
interact favorably for rate and failure count, but they do not repair the
worst-case violation. A successor needs targeted reconstruction refinement or
a decoder-stable lattice that explicitly controls local error; failure-count
reduction alone is insufficient under the updated criteria.

Related: [research 0049](../research/0049-multi-generation-quantization-drift.md),
[research 0050](../research/0050-jpeg-xs-wavelet-perceptual-allocation.md),
[EXP-0178](EXP-0178-absolute-sample-lattice.md), and
[EXP-0182](EXP-0182-median-edge-predictor.md).
