# EXP-0197 — Gray10 step 9 stability probe

Status: **REJECTED**

Date: 2026-07-30

Baseline revision: `7663e5b` (codec source from accepted `1f8ff8c`).
Baseline codec-source SHA-256: `c249ce4bc02dee2eab5f2972ce80b764ac1fd22e634059f74bb6f76ac6e5f1ce`.
Evaluator revision: `ed4febd5b9b2815fc86e1f36e50ef4a63b8eac46`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

## Hypothesis and rationale

At q90, changing gray10's residual reconstruction step from 8 to 9 will reduce
repeated-roundtrip lattice switching on controlling `ai-13-gray-10`, strictly
improve the worst generation violation, and reduce total bytes without failing
ordinary quality. EXP-0196 showed that the ostensibly finer step 7 worsened
`ai-13` generation SSIMULACRA2 from 89.081276 to 87.960106 and Butteraugli
from 2.482678 to 2.867221, establishing a non-monotone response. The coarser
direction is therefore the next falsifiable stability probe.

Only the gray10 denominator changes from 6 to 5, producing step 9 at q90 and
retaining step 1 at q100. Syntax, predictor, entropy coding, accepted gray16
allocation, every other format/depth, and decoder scheduling are unchanged.

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
`25fc29cb6928ec5f1f4b8e2396fa37ecdef060b77ff8b4e3434109df7460d591`.
Candidate patch ID: `c4ee924d6af0066beb345a08269fcab328bcb549`.

- rejection candidate: `evaluation_results/rejection-25fc29cb6928ec5f1f4b8e2396fa37ecdef060b77ff8b4e3434109df7460d591.json`
  (artifact SHA-256 `a94676f95375c7bb41c96704fb3f72e588550dd7859a65026403032e2f0831b9`).

## Result

The candidate encoded 322,184,226 bytes versus 322,248,140 for the baseline,
saving 63,914 bytes (0.020%) and improving ratio from 6.679315x to 6.680640x.
All correctness, determinism, coverage, and performance gates passed.

Quality regressed sharply. Ordinary extrema moved from 94.813339 / 0.747622
to 94.499466 / 0.992124, leaving only 0.007876 Butteraugli headroom. Generation
minimum SSIMULACRA2 fell from 89.081276 to 86.521347 and maximum Butteraugli
rose from 2.482678 to 3.181946. `ai-13-gray-10` controlled every changed
extremum. Failure identities and count remained unchanged at five.

## Conclusion

Rejected at the rejection tier because both worst generation violations became
strictly worse. Steps 7 and 9 both regress `ai-13` relative to accepted step 8,
so scalar step tuning around this cell is exhausted. No full evaluation is
permitted. Source changes were reverted after recording the result.

Related: [EXP-0196](EXP-0196-gray10-step7-rgb16-funding.md) and
[research 0049](../research/0049-multi-generation-quantization-drift.md).
