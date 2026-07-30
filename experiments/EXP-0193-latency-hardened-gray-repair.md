# EXP-0193 — Latency-hardened rate-gated gray repair

Status: **ACCEPTED**

Date: 2026-07-30

Baseline revision: `6398b72` (codec source unchanged from `3ae880c`).
Baseline codec-source SHA-256: `b86e33fecb2f0e7d317f7f621acacaecae618f34f9ade22e739ae9460567680f`.
Evaluator revision: `ed4febd5b9b2815fc86e1f36e50ef4a63b8eac46`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

## Hypothesis and rationale

Retain EXP-0192's canonically proven bimodal gray16 refinement and 4K RGB10
entropy funding, but launch wavefront reconstruction with 128 rather than 256
threads. Default 256x128 tiles have at most 128 samples on any antidiagonal;
the other 128 threads are always idle. Matching the block to useful parallelism
will create enough decode headroom to eliminate EXP-0192's sole 0.504784 ms
latency failure while preserving its 6.37 MB rate saving, quality improvement,
and no-new-quality-failure result.

The rate selector, depth-byte flag, and entropy policy are identical to
EXP-0192. The only successor correction is reconstruction launch geometry;
math, synchronization order, pixels, and syntax are unchanged.

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
`c249ce4bc02dee2eab5f2972ce80b764ac1fd22e634059f74bb6f76ac6e5f1ce`.
Candidate patch ID: `4e0481b91042aa6e434c7e37247271cf52ca03c8`.

- rejection candidate: `evaluation_results/rejection-c249ce4bc02dee2eab5f2972ce80b764ac1fd22e634059f74bb6f76ac6e5f1ce.json`
  (artifact SHA-256 `68e190568b66f1e31aed878689a39b566078a5c4f12099056deac8fd3a6b290e`);
- full candidate: `evaluation_results/full-c249ce4bc02dee2eab5f2972ce80b764ac1fd22e634059f74bb6f76ac6e5f1ce.json`
  (artifact SHA-256 `af852e1fd4412e8215722f0374feb874528b3ed1231cf025a5c2757a28c32e06`).

## Results

The rejection candidate encoded 322,248,140 bytes versus 323,186,668 for the
baseline, saving 938,528 bytes (0.290%) and improving ratio from 6.659918x to
6.679315x. Ordinary extrema remained 94.813339 / 0.747622. Generation minimum
SSIMULACRA2 improved from 88.169777 to 89.081276 while maximum Butteraugli
remained 2.482678. Both artifacts had the same five quality failures. All
correctness, determinism, coverage, and timing gates passed. Candidate 1080p
RGB10 encode/decode medians were 0.968928/0.481456 ms.

The unchanged full candidate encoded 2,123,884,240 bytes versus 2,130,251,655
for the baseline: **6,367,415 fewer bytes** (0.299%), improving ratio from
6.424480x to 6.443741x. Ordinary extrema improved from 86.211967 / 1.632229 to
88.391052 / 1.632229. Generation extrema improved from 81.113586 / 4.645103 to
81.743011 / 4.645103. Failures fell from 174 to 173: the candidate resolved
`procedural-02-gray-16`'s ordinary-quality failure and introduced no new
failure or regression.

All correctness, determinism, coverage, and performance gates passed. Full
candidate 4Kx24 YUV10 encode/decode medians were 58.071/28.633 ms, 4Kx24 RGB10
were 84.174/40.687 ms, and 1080p RGB10 were 0.931/0.474 ms. The 128-thread
wavefront launch therefore cleared the latency gate that rejected EXP-0192,
although it did not improve every throughput measurement.

## Conclusion

Accepted under the failing-baseline exception. The candidate strictly reduces
the worst quality violation, introduces no new failure or regression, reduces
total compressed size, and passes every non-quality gate in the unchanged full
tier. Retain the bimodal gray16 refinement, source-visible depth-byte flag,
4K RGB10 exact entropy competition, and 128-thread reconstruction launch.

Related: [EXP-0192](EXP-0192-bimodal-rate-gated-gray16.md) and
[EXP-0138](EXP-0138-cuda-predictor-schedules.md).
