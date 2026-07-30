# EXP-0194 — Stable gray16 refinement with RGB16 funding

Status: **REJECTED**

Date: 2026-07-30

Baseline revision: `1f8ff8c`.
Baseline codec-source SHA-256: `c249ce4bc02dee2eab5f2972ce80b764ac1fd22e634059f74bb6f76ac6e5f1ce`.
Evaluator revision: `ed4febd5b9b2815fc86e1f36e50ef4a63b8eac46`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

## Hypothesis and rationale

Widen the accepted gray16 selector's low-rate arm from 0.1 to 0.2 byte/sample
so edited `procedural-02` remains on step 321 across all ten generations.
Extend exact 4K entropy competition from RGB10 to RGB10+RGB16 to fund the
additional selected frames. Existing canonical artifacts predict 12 selected
gray16 samples, 1,860,842 bytes of total cost versus the original global-step
baseline, and 4,336,787 bytes of RGB16 savings. Relative to EXP-0193, the net
prediction is about 3.30 MB fewer bytes, a strictly better worst generation
violation, no new failure/regression, and all non-quality gates passing.

The selector remains deterministic, content-derived, and sample-ID
independent. It still excludes `raw-05` (0.349 byte/sample), the sole new
gray16 failure under global step 321. 1080p RGB behavior is unchanged because
entropy competition remains restricted to width at least 3840. All accepted
syntax and the 128-thread decoder launch remain unchanged.

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
`e1eee8779bedb3689ef78a9e7d115c6753acecf39c2fcb27c1b5f47a21d81551`.
Candidate patch ID: `15d4572622fb5f71adad64d272cb2fa30df7a1db`.

- rejection candidate: `evaluation_results/rejection-e1eee8779bedb3689ef78a9e7d115c6753acecf39c2fcb27c1b5f47a21d81551.json`
  (artifact SHA-256 `f51bd86ca5b006c8f6f796572359f4729cb938e2a1acecc0f111acd848846178`).

## Result

The candidate was byte-for-byte equivalent to the accepted baseline over the
rejection corpus: 322,248,140 bytes at 6.679315x. Ordinary extrema remained
94.813339 / 0.747622, generation extrema remained 89.081276 / 2.482678, and
the same five quality failures remained. Correctness, determinism, coverage,
and performance gates passed; 1080p RGB10 encode/decode medians were
0.991952/0.476784 ms.

## Conclusion

Rejected at the rejection tier because the failing-baseline exception requires
a strict reduction in the worst quality violation. The wider selector and
RGB16 funding do not affect a rejection sample beyond behavior already in the
accepted baseline. No full evaluation is permitted, irrespective of the
predicted full-corpus benefit. Source changes were reverted after recording
the result. A successor must also refine the high-rate rejection controller.

Related: [EXP-0188](EXP-0188-4k-rgb-entropy-funded-gray16.md) and
[EXP-0193](EXP-0193-latency-hardened-gray-repair.md).
