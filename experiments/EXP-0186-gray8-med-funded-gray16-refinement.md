# EXP-0186 — Gray8 MED-funded gray16 refinement

Status: **REJECTED**

Date: 2026-07-30

Baseline revision: `534a9a1cdcb856ff911d8c0f2a2755c6a5422570`.
Baseline CUDA SHA-256: `d8f2ab557bb35f70813d3239093db5a322562081f0f21c436f76bc6384694443`.
Candidate source patch ID: `ea74a6085f63f6a5cb7e5b2a7c878e98a15ac43f`.
Candidate CUDA SHA-256: `38f9a8c088f7b81c1295ec40078cf1d7b0e56f7097343e49a87329fe3558bbf5`.
Evaluator revision: `ed4febd5b9b2815fc86e1f36e50ef4a63b8eac46`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

## Hypothesis and change

MED prediction on lossless gray8 changes no reconstructed samples and saved
5,029,425 bytes in the prior matched full artifacts. Spending part of that
guaranteed-quality-neutral gain on a one-code-value gray16 refinement (q90 step
428 to 427) will raise the full baseline's controlling generation minimum above
81.113586, introduce no new failures or quality regressions, and not increase
total compressed size.

The one attributable format/depth policy enables MED for gray8, where step 1
makes reconstruction exact, and reduces every non-lossless gray16 step by one.
All other predictors, steps, entropy modes, metadata, API, corpus, and evaluator
are unchanged.

## Canonical command and artifacts

```sh
PYTHONPATH=.:cuda python3 scripts/evaluate.py --tier rejection \
  --output <artifact> --libvship-revision v5.0.0 \
  --libvship-build '5.0.0 CUDA direct C API' \
  --libvship-gpu-id 0 --libvship-workers 2
```

The baseline CUDA checksum was a cache hit; no baseline was rerun:

- rejection baseline: `evaluation_results/rejection-d8f2ab557bb35f70813d3239093db5a322562081f0f21c436f76bc6384694443.json`;
- full baseline: `evaluation_results/full-d8f2ab557bb35f70813d3239093db5a322562081f0f21c436f76bc6384694443.json`.

- candidate rejection: `evaluation_results/rejection-38f9a8c088f7b81c1295ec40078cf1d7b0e56f7097343e49a87329fe3558bbf5.json`.

## Result

The focused evaluator/API suite passed all 35 tests.

| Codec | Bytes | Ratio | Ordinary min/max | Generation min/max | Failures |
|---|---:|---:|---:|---:|---:|
| baseline | 323,186,668 | 6.659918x | 94.813339 / 0.747622 | 88.169777 / 2.482678 | 5 |
| candidate | 323,152,713 | 6.660618x | 94.813339 / 0.747622 | 87.482933 / 2.482678 | 5 |

The candidate saved 33,955 bytes and introduced no new failure identity, but
the controlling generation SSIMULACRA2 regressed by 0.686844. Candidate artifact
SHA-256 is
`14032f2f9b0a36fd2c61d6e93645eb0bce0f805d5a33bc75e13d70bc78190331`.
Full was not run because the rejection comparison failed.

## Conclusion

Reject and restore the accepted EXP-0185 baseline. A one-code-value finer
gray16 scalar step is not monotonically safer after ten edit generations. The
lossless gray8 MED component remains independently promising, but it cannot be
accepted alone while the baseline is failing because it leaves the worst
quality violation unchanged.

Related: [EXP-0184](EXP-0184-med-extrema-repair.md),
[EXP-0185](EXP-0185-nongray-med-extrema-repair.md).
