# EXP-0195 — Three-region gray16 allocation

Status: **REJECTED**

Date: 2026-07-30

Baseline revision: `21e8774` (codec source from accepted `1f8ff8c`).
Baseline codec-source SHA-256: `c249ce4bc02dee2eab5f2972ce80b764ac1fd22e634059f74bb6f76ac6e5f1ce`.
Evaluator revision: `ed4febd5b9b2815fc86e1f36e50ef4a63b8eac46`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

## Hypothesis and rationale

Use three deterministic gray16 reconstruction regions derived from complete
baseline residual rate: step 321 below 0.2 byte/sample, accepted step 428 in
the middle region, and aligned step 257 above 0.5 byte/sample. The low arm will
keep edited `procedural-02` on a stable lattice; the high arm will strictly
improve rejection controller `procedural-03`; and the middle arm will exclude
`raw-05` and the natural-content regressions. Extending exact 4K entropy
competition to RGB16 will fund the additional precision. The unchanged
candidate will improve the worst violation, add no failure/regression, reduce
bytes, and pass every non-quality gate.

Two previously unused high bits in the existing depth byte signal the two
refinement classes with no metadata increase. Step 257 is the denominator-10
gray16 lattice and exactly divides 65535, aligning endpoints and 8-bit-expanded
values. All accepted predictors, entropy syntax, and decoder launch geometry
remain unchanged.

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
`f50e8acd8625daa0b02a9cd73704673b6f6f1a246d55bffb1f67c15d937ed8e3`.
Candidate patch ID: `b827fbfda7aa87c794cc9cccd50cba4ecd60fab6`.

- rejection candidate: `evaluation_results/rejection-f50e8acd8625daa0b02a9cd73704673b6f6f1a246d55bffb1f67c15d937ed8e3.json`
  (artifact SHA-256 `0fdaa057d34d79875671b1506b2114b55fc57e409bf5d38cca0ba61500b2538e`).

## Result

The candidate resolved `procedural-03-gray-16`'s generation failure, reducing
failures from five to four. It nevertheless encoded 322,627,667 bytes versus
322,248,140 for the accepted baseline: **379,527 more bytes** (+0.118%), and
ratio fell from 6.679315x to 6.671458x. Global extrema remained unchanged at
ordinary 94.813339 / 0.747622 and generation 89.081276 / 2.482678 because
`ai-13-gray-10` controls both generation extrema. Correctness, determinism,
coverage, and all performance gates passed; 1080p RGB10 encode/decode medians
were 0.944256/0.474640 ms.

## Conclusion

Rejected at the rejection tier. It increases total bytes and does not strictly
reduce the global worst quality violation, even though one secondary failure
is resolved. No full evaluation is permitted. The next quality allocation must
target controlling gray10 rather than spend more bits only on gray16. Source
changes were reverted after recording the result.

Related: [EXP-0193](EXP-0193-latency-hardened-gray-repair.md) and
[EXP-0194](EXP-0194-stable-gray16-rgb16-funding.md).
