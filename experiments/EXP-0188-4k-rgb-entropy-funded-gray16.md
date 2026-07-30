# EXP-0188 — 4K RGB entropy-funded gray16 repair

Status: **REJECTED**

Date: 2026-07-30

Baseline revision: `fdea4fa`.
Baseline CUDA SHA-256: `99d07076b172ab20472ede5100f2d5847db6fe2b7814cf229c63f59737b52509`.
Candidate source patch ID: `b5b11cabeabc67bf1ef22925db91c3db399aef2f`.
Candidate CUDA SHA-256: `c57ebee2cdbf85bbe65040de8a6bf4bb0614c80b394232a1967ffdc1e21c21d3`.
Evaluator revision: `ed4febd5b9b2815fc86e1f36e50ef4a63b8eac46`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

## Hypothesis and change

Allowing the existing Rice/zero-run/block-pack modes to compete with order-0
rANS only on 4K RGB frames will provide a quality-neutral rate gain large
enough to fund gray16 q90 step 321 instead of 428. The combined rate-quality
allocation will strictly improve the controlling generation SSIMULACRA2,
introduce no new failures or regressions, remain within performance gates, and
not increase total bytes.

Latency-sensitive sub-4K RGB remains forced to order-0. The accepted encoder's
4K RGB throughput is 2.61 GP/s against a 1.5 GP/s gate, providing bounded
analysis headroom. Gray reconstruction changes only through its fixed step;
all predictors, bitstream syntax, decoder entropy paths, and other format/depth
steps are unchanged.

## Canonical command and artifacts

```sh
PYTHONPATH=.:cuda python3 scripts/evaluate.py --tier rejection \
  --output <artifact> --libvship-revision v5.0.0 \
  --libvship-build '5.0.0 CUDA direct C API' \
  --libvship-gpu-id 0 --libvship-workers 2
```

The baseline checksum was a cache miss after rebuild and was evaluated once:

- rejection baseline: `evaluation_results/rejection-99d07076b172ab20472ede5100f2d5847db6fe2b7814cf229c63f59737b52509.json`.

It encoded 323,186,668 bytes at 6.659918x, with ordinary extrema
94.813339 / 0.747622, generation extrema 88.169777 / 2.482678, and five
failures.

- candidate rejection: `evaluation_results/rejection-c57ebee2cdbf85bbe65040de8a6bf4bb0614c80b394232a1967ffdc1e21c21d3.json`;
- full baseline: `evaluation_results/full-cf144173a3185e7dbb2919671537550fd20b35ceb42b6f3b6370f42b76d71247.json`;
- unchanged candidate full: `evaluation_results/full-c57ebee2cdbf85bbe65040de8a6bf4bb0614c80b394232a1967ffdc1e21c21d3.json`.

## Results

The focused evaluator/API suite passed all 35 tests.

| Tier / codec | Bytes | Ratio | Ordinary min/max | Generation min/max | Failures |
|---|---:|---:|---:|---:|---:|
| rejection baseline | 323,186,668 | 6.659918x | 94.813339 / 0.747622 | 88.169777 / 2.482678 | 5 |
| rejection candidate | 322,248,140 | 6.679315x | 94.813339 / 0.747622 | 89.081276 / 2.482678 | 5 |
| full baseline | 2,130,251,655 | 6.424480x | 86.211967 / 1.632229 | 81.113586 / 4.645103 | 174 |
| full candidate | 2,128,983,896 | 6.428306x | 88.391052 / 1.632229 | 85.946732 / 4.645103 | 169 |

The candidate saved 1,267,759 full-tier bytes, strictly improved both SSIM
extrema, preserved both Butteraugli extrema, and removed seven baseline
failures. It introduced two new failures: `raw-05-gray-16` generation quality
and `ai-10-rgb444-16` encode latency. Candidate artifacts hash to
`e31a112b43becacae0b03bac9d76b8a58b9a42d6d60455e86aa5dca9f9641056`
(rejection) and
`649806fb1b8d570ba4f4f950fd72e2ce6c4891f31fb9852f1ba9cccd8b7eb07c`
(full).

## Conclusion

Reject on the two new full-tier failures and restore EXP-0185. The 4K entropy
competition provides sufficient aggregate funding and step 321 materially
repairs the global extrema, but RGB16 must remain on its faster order-0 path and
gray16 needs a reconstruction rule that does not newly fail `raw-05`.

Related: [EXP-0180](EXP-0180-lossless-entropy-competition.md),
[EXP-0187](EXP-0187-gray-med-recenter-step321.md).
