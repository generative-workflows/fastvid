# EXP-0187 — Reversible gray MED recenter with step-321 gray16

Status: **REJECTED**

Date: 2026-07-30

Baseline revision: `9cb1d11`.
Baseline CUDA SHA-256: `3500284ba4bb3b7aed839c2af3123eeed2c6a2efc7959a14c225cb70a3cc24ea`.
Candidate source patch ID: `d0ae0f1b4ef946767eb5784690e3649edf9f4fb6`.
Candidate CUDA SHA-256: `462ad3c8b1456704d9eba3727fbaf6178399a7faf3f7325d09d66c599e6f811b`.
Evaluator revision: `ed4febd5b9b2815fc86e1f36e50ef4a63b8eac46`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

## Hypothesis and change

Reversibly recentering gray quantizer indices around MED will preserve the
accepted clamped-gradient reconstruction exactly at unchanged steps while
recovering enough gray entropy to fund a materially finer gray16 q90 step
(428 to 321). The combined gray coding policy will improve the controlling
multi-generation SSIMULACRA2 violation, add no failures/regressions, and not
increase bytes.

For gray, the encoder still derives quantized reconstruction from the clamped
gradient. It transmits the quantizer index minus a decoder-reproducible rounded
MED-to-gradient offset. The decoder adds that offset back and performs the
unchanged gradient reconstruction. This is an integer bijection; unlike direct
MED quantization, it cannot alter gray10 reconstruction. Gray16 uses denominator
8 instead of 6 at q90. Nongray coding is unchanged.

## Canonical command and artifacts

```sh
PYTHONPATH=.:cuda python3 scripts/evaluate.py --tier rejection \
  --output <artifact> --libvship-revision v5.0.0 \
  --libvship-build '5.0.0 CUDA direct C API' \
  --libvship-gpu-id 0 --libvship-workers 2
```

The rebuilt baseline checksum was a cache miss, so its rejection tier was run:

- rejection baseline: `evaluation_results/rejection-3500284ba4bb3b7aed839c2af3123eeed2c6a2efc7959a14c225cb70a3cc24ea.json`.

It reproduces EXP-0185: 323,186,668 bytes, 6.659918x, ordinary extrema
94.813339 / 0.747622, generation extrema 88.169777 / 2.482678, and five
failures.

- candidate rejection: `evaluation_results/rejection-462ad3c8b1456704d9eba3727fbaf6178399a7faf3f7325d09d66c599e6f811b.json`.

## Result

The focused evaluator/API suite passed all 35 tests.

| Codec | Bytes | Ratio | Ordinary min/max | Generation min/max | Failures |
|---|---:|---:|---:|---:|---:|
| baseline | 323,186,668 | 6.659918x | 94.813339 / 0.747622 | 88.169777 / 2.482678 | 5 |
| candidate | 323,945,965 | 6.644308x | 94.813339 / 0.747622 | 89.081276 / 2.482678 | 5 |

The candidate improved the controlling generation SSIMULACRA2 by 0.911499 and
introduced no new failure identity, but expanded output by 759,297 bytes
(0.235%). Artifact SHA-256 is
`ae253dc052a3d302bd2f616f966b5c205f09d0c218e4f01affc5b1e9b232eb08`.
Full was not run because the rejection comparison failed the no-size-increase
criterion.

## Conclusion

Reject and restore EXP-0185. Reversible MED recentering is reconstruction-safe,
and step 321 improves the measured worst case, but the realized entropy gain on
the rejection corpus does not fund that precision. A successor needs a larger
independent entropy gain or a transform-domain allocation mechanism rather than
further scalar-step tuning.

Related: [EXP-0185](EXP-0185-nongray-med-extrema-repair.md),
[EXP-0186](EXP-0186-gray8-med-funded-gray16-refinement.md).
