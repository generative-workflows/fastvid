# EXP-0185 — Non-gray MED with targeted YUV extrema repair

Status: **ACCEPTED**

Date: 2026-07-30

Baseline revision: `7664ed67c8509113e6a6d0ce1b8e60481c132234`.
Baseline CUDA SHA-256: `c80229390f9376ff3e572a3191fb532e21e770faa55c0cb465f13b86507b45e7`.
Candidate source patch ID: `74a74a052f059facebe61375662d1be86c6222d7`.
Candidate CUDA SHA-256: `d8f2ab557bb35f70813d3239093db5a322562081f0f21c436f76bc6384694443`.
Evaluator revision: `ed4febd5b9b2815fc86e1f36e50ef4a63b8eac46`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

## Hypothesis and change

EXP-0184's only new full-tier failures were gray samples caused by MED. Using
the baseline clamped-gradient predictor for all gray frames while retaining MED
for YUV/RGB and the targeted YUV extrema steps will remove those new failures,
strictly improve the full baseline's worst quality violation, and retain a net
full-corpus byte saving.

The one attributable change relative to EXP-0184 is format-selective prediction:
gray uses the baseline predictor; YUV/RGB use MED. At q90 YUV422-8 remains
lossless step 1 and YUV422-16 uses step 108. Other quantizer steps, entropy,
metadata, API, corpus, and evaluator are unchanged.

## Canonical command and artifacts

```sh
PYTHONPATH=.:cuda python3 scripts/evaluate.py --tier rejection \
  --output <artifact> --libvship-revision v5.0.0 \
  --libvship-build '5.0.0 CUDA direct C API' \
  --libvship-gpu-id 0 --libvship-workers 2
```

The unchanged baseline was a cache hit and was not rerun:

- rejection baseline: `evaluation_results/rejection-c80229390f9376ff3e572a3191fb532e21e770faa55c0cb465f13b86507b45e7.json`;
- full baseline: `evaluation_results/full-c80229390f9376ff3e572a3191fb532e21e770faa55c0cb465f13b86507b45e7.json`;
- candidate rejection: `evaluation_results/rejection-d8f2ab557bb35f70813d3239093db5a322562081f0f21c436f76bc6384694443.json`;
- unchanged candidate full: `evaluation_results/full-d8f2ab557bb35f70813d3239093db5a322562081f0f21c436f76bc6384694443.json`.

The cached rejection baseline encoded 347,833,953 bytes at 6.188001x, with
ordinary extrema 93.697319 / 0.803438, generation extrema 87.446571 / 2.702818,
and nine failures.

## Results

The focused evaluator/API suite passed all 35 tests.

| Tier / codec | Bytes | Ratio | Ordinary min/max | Generation min/max | Failures |
|---|---:|---:|---:|---:|---:|
| rejection baseline | 347,833,953 | 6.188001x | 93.697319 / 0.803438 | 87.446571 / 2.702818 | 9 |
| rejection candidate | 323,186,668 | 6.659918x | 94.813339 / 0.747622 | 88.169777 / 2.482678 | 5 |
| full baseline | 2,145,624,455 | 6.378451x | 73.365837 / 4.157670 | 67.487183 / 6.425947 | 274 |
| full candidate | 2,130,251,655 | 6.424480x | 86.211967 / 1.632229 | 81.113586 / 4.645103 | 174 |

The unchanged candidate saved 15,372,800 full-tier bytes (0.716%), strictly
improved every full-corpus quality extremum, removed 100 recorded failures, and
introduced no new failures. Correctness, coverage, determinism, and every
performance gate passed. Cached artifact SHA-256 values are
`2bb6359f07678935cea7930044ceb40c3606706610e55144198b09a4ef3dd701`
(rejection) and
`b055a3fbdba498e789f7ece7e4ddec1dc4f411bec7bdfa7065dcf8cb43668784`
(full).

## Conclusion

Accept under the failing-baseline exception. Format-selective prediction keeps
gray byte-for-byte on its baseline reconstruction path, eliminating EXP-0184's
two new failures, while MED retains useful YUV/RGB rate gains. Lossless YUV8
and finer YUV16 repair the original full-corpus worst extrema. This accepted
candidate becomes the new pre-v1 baseline.

Related: [EXP-0182](EXP-0182-median-edge-predictor.md),
[EXP-0184](EXP-0184-med-extrema-repair.md).
