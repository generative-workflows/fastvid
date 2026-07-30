# EXP-0196 — Gray10 step 7 with RGB16 entropy funding

Status: **REJECTED**

Date: 2026-07-30

Baseline revision: `cdf68a2` (codec source from accepted `1f8ff8c`).
Baseline codec-source SHA-256: `c249ce4bc02dee2eab5f2972ce80b764ac1fd22e634059f74bb6f76ac6e5f1ce`.
Evaluator revision: `ed4febd5b9b2815fc86e1f36e50ef4a63b8eac46`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

## Hypothesis and rationale

At q90, refining gray10 from step 8 to step 7 will strictly improve both
generation extrema controlled by `ai-13-gray-10`. Allowing the existing exact
entropy competition for RGB16 at every resolution will fund the rate cost.
The candidate will introduce no new failure/regression, not increase bytes,
and pass every correctness, coverage, determinism, and performance gate.

The gray10 formula changes only its denominator from 6 to 7; q100 remains
lossless. RGB10 retains the accepted 4K-only competition policy, while RGB16
compares complete legacy and order-0 shard bytes at all widths. Prediction,
syntax, accepted gray16 allocation, and decoder reconstruction are unchanged.

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
`4e1d2d3ef92d2229662c99075c2f8854fdce01b576401f0419ead30efb33744a`.
Candidate patch ID: `ffdd4145e8b59479b7a0fa19ef8f84951ca0b289`.

- rejection candidate: `evaluation_results/rejection-4e1d2d3ef92d2229662c99075c2f8854fdce01b576401f0419ead30efb33744a.json`
  (artifact SHA-256 `0c3d64384ab87b71df38b4d40eec9e4610285daeb15777f1840b92734267a8d5`).

## Result

The candidate encoded 322,177,984 bytes versus 322,248,140 for the baseline,
saving 70,156 bytes (0.022%) and improving ratio from 6.679315x to 6.680769x.
Ordinary maximum Butteraugli improved from 0.747622 to 0.699139 and the
ordinary SSIMULACRA2 floor remained 94.813339.

Generation quality moved in the wrong direction: minimum SSIMULACRA2 fell
from 89.081276 to 87.960106 and maximum Butteraugli rose from 2.482678 to
2.867221. Both regressions are controlled by `ai-13-gray-10`, showing another
non-monotone repeated-roundtrip response to a finer scalar step. The candidate
also introduced `ai-05-rgb444-16: decode latency >= 0.5 ms` at 0.539008 ms
versus the baseline artifact's 0.464096 ms. Correctness and determinism passed.

## Conclusion

Rejected at the rejection tier. It worsens both worst generation-quality
violations and fails a performance gate, independently violating acceptance.
No full evaluation is permitted. Gray10 requires a stable reconstruction
lattice or content-adaptive class rather than a global finer residual step.
Source changes were reverted after recording the result.

Related: [EXP-0193](EXP-0193-latency-hardened-gray-repair.md) and
[EXP-0195](EXP-0195-three-region-gray16-allocation.md).
