# EXP-0191 — Rate-gated gray16 refinement

Status: **REJECTED**

Date: 2026-07-30

Baseline revision: `8f928fd` (codec source unchanged from `3ae880c`).
Baseline codec-source SHA-256: `b86e33fecb2f0e7d317f7f621acacaecae618f34f9ade22e739ae9460567680f`.
Evaluator revision: `ed4febd5b9b2815fc86e1f36e50ef4a63b8eac46`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

## Hypothesis and rationale

A decoder-signaled step-321 refinement restricted to gray16 frames whose
baseline residual payload is below 0.1 byte/sample will improve the controlling
`procedural-02-gray-16` quality violation without touching the natural-content
frames that regressed under global refinement. Exact entropy competition only
for 4K RGB444-10 will fund the small selected-frame rate cost. The unchanged
candidate will reduce bytes, strictly reduce the worst quality violation,
introduce no new failure/regression, and pass every non-quality gate.

The selector is content-derived, deterministic, and sample-ID independent.
Existing full-tier evidence shows the threshold admits `procedural-02` (59.30x
against 16-bit raw) and two essentially constant HDRI frames (905.65x), while
the observed global-step regression cases range from 5.73x to 7.27x. The
encoder measures complete baseline shard bytes, reruns only admitted gray16
frames at step 321, and signals the refinement in the unused high bit of the
existing depth byte. There is no metadata-size increase. All other frames and
decoder reconstruction paths remain unchanged.

## Canonical command and artifacts

```sh
PYTHONPATH=.:cuda python3 scripts/evaluate.py --tier <rejection|full> \
  --output <artifact> --libvship-revision v5.0.0 \
  --libvship-build '5.0.0 CUDA direct C API' \
  --libvship-gpu-id 0 --libvship-workers 2
```

- rejection baseline cache hit: `evaluation_results/rejection-b86e33fecb2f0e7d317f7f621acacaecae618f34f9ade22e739ae9460567680f.json`;
- full baseline cache hit if required: `evaluation_results/full-b86e33fecb2f0e7d317f7f621acacaecae618f34f9ade22e739ae9460567680f.json`.

Candidate codec-source SHA-256:
`a66941a05bab7792b2dbfd98dc7f73154d9de1978a792581918a924c0bbbcb62`.
Candidate patch ID: `f28e2c935abfcc53332e49311958ccffa49eec61`.

- rejection candidate: `evaluation_results/rejection-a66941a05bab7792b2dbfd98dc7f73154d9de1978a792581918a924c0bbbcb62.json`
  (artifact SHA-256 `7fa7ac82c4a86a9507f5542290954832756fbabc0706b4ec788a1841532326bc`).

## Result

The candidate encoded 321,775,668 bytes versus 323,186,668 for the baseline,
saving 1,411,000 bytes (0.437%) and improving ratio from 6.659918x to
6.689122x. However, all four quality extrema were unchanged: ordinary
94.813339 / 0.747622 and generation 88.169777 / 2.482678. The selector did not
admit rejection controller `procedural-03-gray-16`, whose baseline payload is
0.695 byte/sample, so it did not strictly reduce the worst quality violation.

The candidate also introduced
`performance-1080p-rgb444-10: encode latency >= 1.0 ms`: median encode latency
rose from 0.944752 ms to 1.015136 ms. Candidate 4Kx24 YUV10 encode/decode
medians were 59.374/27.706 ms; 4Kx24 RGB10 medians were 85.041/39.656 ms.
Correctness, determinism, coverage, and all other timing gates passed.

## Conclusion

Rejected at the rejection tier. It neither strictly improves the worst quality
violation nor passes every performance gate. No full evaluation is permitted.
The redundant first-pass tensor clears account for avoidable encoder work; a
successor must also admit high-rate procedural gray16 content while excluding
`raw-05`. Source changes were reverted after recording the result.

Related: [EXP-0188](EXP-0188-4k-rgb-entropy-funded-gray16.md),
[EXP-0189](EXP-0189-aligned-gray16-rgb10-funding.md),
[EXP-0190](EXP-0190-gray16-step417-rgb10-funding.md), and
[research 0049](../research/0049-multi-generation-quantization-drift.md).
