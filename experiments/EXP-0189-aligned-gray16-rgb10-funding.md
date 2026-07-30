# EXP-0189 — Aligned gray16 lattice with RGB10-only funding

Status: **REJECTED**

Date: 2026-07-30

Baseline revision: `3ae880c`.
Baseline codec-source SHA-256: `b86e33fecb2f0e7d317f7f621acacaecae618f34f9ade22e739ae9460567680f`.
Candidate codec-source SHA-256: `8ce46908b15e5b63c7def00e4da8ddcf5c2369ed3e3e666c8e36c0b409942430`.
Candidate patch ID: `2a012ff248be780b46cedd63447d180df89fee6c`.
Evaluator revision: `ed4febd5b9b2815fc86e1f36e50ef4a63b8eac46`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

## Hypothesis and change

Using an aligned gray16 q90 lattice step of 320 (a multiple of 64) instead of
EXP-0188's ceil-derived 321 will retain the large improvement to the controlling
gray16 case without creating `raw-05`'s generation Butteraugli failure.
Restricting 4K entropy competition to RGB10 will preserve its funding while
eliminating EXP-0188's RGB16 latency regression. The candidate will introduce
no new failure/regression and not increase bytes.

This is one corrected rate-quality allocation policy. Gray16 first computes the
denominator-8 step and subtracts one when non-lossless. Only RGB444-10 frames at
width at least 3840 allow the existing exact per-shard entropy competition.
Predictors, syntax, decoder entropy paths, all other steps, corpus, and evaluator
are unchanged.

## Canonical artifacts

```sh
PYTHONPATH=.:cuda python3 scripts/evaluate.py --tier <rejection|full> \
  --output <artifact> --libvship-revision v5.0.0 \
  --libvship-build '5.0.0 CUDA direct C API' \
  --libvship-gpu-id 0 --libvship-workers 2
```

The source checksum is the SHA-256 of the sorted path-and-content SHA-256
manifest of tracked files under `fastvid/` and `cuda/`. The unchanged baseline
artifacts were cache hits after migrating the existing canonical results from
the obsolete binary-checksum cache key.

- rejection baseline: `evaluation_results/rejection-b86e33fecb2f0e7d317f7f621acacaecae618f34f9ade22e739ae9460567680f.json`
  (artifact SHA-256 `366be20858697ba74a55daf5d10a04fc3a98168c6051fc8d69e08e267b61d1db`);
- rejection candidate: `evaluation_results/rejection-8ce46908b15e5b63c7def00e4da8ddcf5c2369ed3e3e666c8e36c0b409942430.json`
  (artifact SHA-256 `327c8d86c09e194d3dbbaa2960add60b173c7b8e2e68eab297570eb9f17fcb4b`);
- full baseline: `evaluation_results/full-b86e33fecb2f0e7d317f7f621acacaecae618f34f9ade22e739ae9460567680f.json`
  (artifact SHA-256 `bf89f4e845b12e6741742be98294652237232595754760788e879cfbc85cd653`);
- full candidate: `evaluation_results/full-8ce46908b15e5b63c7def00e4da8ddcf5c2369ed3e3e666c8e36c0b409942430.json`
  (artifact SHA-256 `1cfdb8651c1a8fee2ab7f55e0df03d10f1f18a3ff487901c7d791f194b39df53`).

## Results

The rejection baseline encoded 323,186,668 bytes at 6.659918x. The candidate
encoded 322,253,234 bytes at 6.679209x, saving 933,434 bytes. Ordinary extrema
were unchanged at 94.813339 / 0.747622. The generation SSIMULACRA2 floor
improved from 88.169777 to 89.081276 while the Butteraugli maximum remained
2.482678. Both artifacts had the same five quality failures and no correctness,
coverage, determinism, or performance failures, so the candidate qualified for
full evaluation.

The full baseline encoded 2,130,251,655 bytes at 6.424480x. The unchanged
candidate encoded 2,133,523,596 bytes at 6.414628x: **3,271,941 more bytes**
(+0.154%). Ordinary extrema improved from 86.211967 / 1.632229 to
88.391052 / 1.632229. Generation extrema improved from 81.113586 / 4.645103 to
85.946732 / 4.645103. Failures fell from 174 to 168, but the candidate
introduced `raw-05-gray-16: generation robustness quality gate failed` while
resolving seven other failures.

All correctness and determinism checks passed. The performance samples also
passed: candidate 4Kx24 YUV10 encode/decode throughput was 3.434/7.141 GP/s,
4Kx24 RGB10 was 2.388/5.031 GP/s, and 1080p RGB10 encode/decode median latency
was 0.928/0.478 ms.

## Conclusion

Rejected. The full-tier size increase violates the non-increase requirement,
and the new `raw-05-gray-16` failure independently violates the no-new-failures
requirement. The rejection subset incorrectly suggested that RGB10 entropy
competition could fund the gray16 refinement corpus-wide. Source changes were
reverted after recording the result.

Related: [EXP-0188](EXP-0188-4k-rgb-entropy-funded-gray16.md).
