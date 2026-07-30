# EXP-0184 — MED with targeted extrema repair

Status: **REJECTED**

Date: 2026-07-30

Baseline revision: `a741a761d31ece3223c57b698a656608a9e0e558`.
Candidate source patch ID: `41ec8d7dfefb9f23231d167627ce144dafde1677`.
Evaluator revision: `ed4febd5b9b2815fc86e1f36e50ef4a63b8eac46`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

## Hypothesis and change

MED prediction's measured 8.13% rate saving can fund precision only in the two
format/depth cells that define the failing baseline's global generation
extrema. At q90, YUV422-8 changes from step 3 to lossless step 1 and YUV422-16
changes from step 215 to step 108. All other cells retain their current steps.

The candidate will make both global generation extrema strictly better than
the baseline (SSIMULACRA2 above 87.446571 and Butteraugli below 2.702818), add
no failures or gate regressions, and remain at or below 347,833,953 bytes.

This is one attributable reconstruction policy: use MED prediction globally
and allocate finer causal-residual quantization only to the measured worst
YUV cells. It intentionally excludes EXP-0183's absolute lattice, which caused
the gray10 regression. Entropy coding, metadata, API, corpus, and evaluator are
unchanged.

## Canonical command and artifacts

```sh
PYTHONPATH=.:cuda python3 scripts/evaluate.py --tier rejection \
  --output /tmp/fastvid-exp0184-{baseline,candidate}-rejection.json \
  --libvship-revision v5.0.0 \
  --libvship-build '5.0.0 CUDA direct C API' \
  --libvship-gpu-id 0 --libvship-workers 2
```

- baseline: `/tmp/fastvid-exp0184-baseline-rejection.json`;
- candidate: `/tmp/fastvid-exp0184-candidate-rejection.json`.

## Baseline

The freshly rebuilt baseline encoded 347,833,953 bytes at 6.188001x. Ordinary
quality extrema were 93.697319 SSIMULACRA2 and 0.803438 Butteraugli. Generation
extrema were 87.446571 and 2.702818, with nine failing samples.

## Results

The focused evaluator/API suite passed all 35 tests.

| Tier / codec | Bytes | Ratio | Ordinary min/max | Generation min/max | Failures |
|---|---:|---:|---:|---:|---:|
| rejection baseline | 347,833,953 | 6.188001x | 93.697319 / 0.803438 | 87.446571 / 2.702818 | 9 |
| rejection candidate | 323,451,859 | 6.654458x | 94.813339 / 0.747542 | 89.327042 / 2.308000 | 4 |
| full baseline | 2,145,624,455 | 6.378451x | 73.365837 / 4.157670 | 67.487183 / 6.425947 | 274 |
| full candidate | 2,111,377,877 | 6.481909x | 86.212021 / 1.632229 | 81.113586 / 4.645101 | 171 |

Full artifacts:

- baseline: `/tmp/fastvid-exp0184-baseline-full.json`;
- unchanged candidate: `/tmp/fastvid-exp0184-candidate-full.json`.

The candidate saved 34,246,578 full-tier bytes (1.60%), strictly improved all
four full-corpus quality extrema, and removed 105 recorded failures. However,
it introduced two new generation failures: `raw-40-gray-10` and
`raw-05-gray-16`. All correctness, coverage, determinism, and performance gates
otherwise passed.

## Conclusion

Reject because the full-tier comparison introduced new failures. Restore the
baseline codec. A successor should retain the targeted YUV precision and MED
for YUV/RGB, but use the baseline clamped-gradient predictor for gray. Full-tier
accounting shows gray MED supplied 18,873,778 of the savings; removing it still
leaves an estimated 15.4 MB rate improvement while making gray reconstruction
identical to the baseline and therefore eliminating both new failures.

Related: [EXP-0176](EXP-0176-edit-resilient-quantizer-map.md),
[EXP-0182](EXP-0182-median-edge-predictor.md), and
[EXP-0183](EXP-0183-med-absolute-lattice.md).
