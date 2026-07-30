# EXP-0192 — Bimodal rate-gated gray16 refinement

Status: **REJECTED**

Date: 2026-07-30

Baseline revision: `012c6ee` (codec source unchanged from `3ae880c`).
Baseline codec-source SHA-256: `b86e33fecb2f0e7d317f7f621acacaecae618f34f9ade22e739ae9460567680f`.
Evaluator revision: `ed4febd5b9b2815fc86e1f36e50ef4a63b8eac46`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

## Hypothesis and rationale

Refine gray16 to step 321 only when its baseline payload is either below 0.1
or above 0.5 byte/sample. This bimodal selector admits both controlling
procedural samples and two constant-image cases while excluding `raw-05`, the
sole new gray16 failure under EXP-0188's global refinement. Existing canonical
full artifacts predict seven selected samples and 824,959 added gray16 bytes.
4K RGB10 entropy competition previously saved 7,192,374 bytes, predicting a
net reduction near 6.37 MB. The candidate will strictly reduce the worst
quality violation, introduce no new failures/regressions, reduce total bytes,
and pass every non-quality gate.

The content-derived selector uses complete baseline shard bytes and is
independent of sample identity. Selected streams signal refinement in the
unused high bit of the existing depth byte, adding no metadata. Unlike
EXP-0191, the first analysis pass does not clear already-initialized tensors;
only the rare second pass resets reconstruction and status state. Syntax,
predictors, and all unselected reconstruction paths remain unchanged.

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
`f91d04d8134bea5013a8f7720a5875bf15bf7dce57e7fed6de090b2df4dc1792`.
Candidate patch ID: `55aa951c827e4c9c1c571babf3967ca92b6eddd1`.

- rejection candidate: `evaluation_results/rejection-f91d04d8134bea5013a8f7720a5875bf15bf7dce57e7fed6de090b2df4dc1792.json`
  (artifact SHA-256 `044d2e52800a12f5c148c8a33c33eeaea2da8a604ba2b536ca2624a328362a90`);
- full candidate: `evaluation_results/full-f91d04d8134bea5013a8f7720a5875bf15bf7dce57e7fed6de090b2df4dc1792.json`
  (artifact SHA-256 `e7a271a72a319a0698027bd88bf0fc080d38684ea2082fc6734c60fa4852a2d8`).

## Results

The rejection candidate encoded 322,248,140 bytes versus 323,186,668 for the
baseline, saving 938,528 bytes (0.290%) and improving ratio from 6.659918x to
6.679315x. Ordinary extrema remained 94.813339 / 0.747622. Generation minimum
SSIMULACRA2 improved from 88.169777 to 89.081276 while maximum Butteraugli
remained 2.482678. Both artifacts had exactly the same five quality failures.
Correctness, determinism, coverage, and all timing gates passed; 1080p RGB10
encode/decode medians were 0.951824/0.479520 ms.

The full candidate encoded 2,123,884,240 bytes versus 2,130,251,655 for the
baseline: **6,367,415 fewer bytes** (0.299%), improving ratio from 6.424480x to
6.443741x. Ordinary extrema improved from 86.211967 / 1.632229 to
88.391052 / 1.632229. Generation extrema improved from 81.113586 / 4.645103 to
81.743011 / 4.645103. Failure count remained 174. The candidate resolved
`procedural-02-gray-16`'s ordinary-quality failure and introduced no new
quality failure.

However, it introduced the sole new failure
`ai-04-rgb444-16: decode latency >= 0.5 ms`. The canonical median was
0.504784 ms versus the baseline artifact's 0.468640 ms. That sample's pixels,
bitstream path, quality (96.984962 / 0.412102), and deterministic correctness
were unchanged; nevertheless, the recorded performance gate is binding.

## Conclusion

Rejected because any performance failure mandates rejection. The rate-gated
gray repair itself met its predicted rate, worst-quality, and no-new-quality-
failure objectives, saving 6.37 MB. A successor requires measured decode
headroom rather than another selector change. Source changes were reverted
after recording the result.

Related: [EXP-0188](EXP-0188-4k-rgb-entropy-funded-gray16.md),
[EXP-0191](EXP-0191-rate-gated-gray16-refinement.md), and
[research 0049](../research/0049-multi-generation-quantization-drift.md).
