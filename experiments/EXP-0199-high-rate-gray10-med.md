# EXP-0199 — High-rate gray10 MED predictor

Status: **REJECTED**

Date: 2026-07-30

Baseline revision: `88d638a` (codec source retained from accepted EXP-0193).
Baseline source SHA-256: `c249ce4bc02dee2eab5f2972ce80b764ac1fd22e634059f74bb6f76ac6e5f1ce`.
Evaluator revision: `ed4febd5b9b2815fc86e1f36e50ef4a63b8eac46`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

## Hypothesis and change

Gray MED supplied 18,873,778 bytes of full-corpus savings in EXP-0184, but
unconditional gray MED introduced `raw-40-gray-10` and `raw-05-gray-16`
generation failures. EXP-0185 therefore restored clamped-gradient prediction
for every gray frame. The remaining global rejection violation is now
`ai-13-gray-10`, whose accepted baseline payload is 0.327 bytes/sample, while
the known MED regression `raw-40-gray-10` is lower-rate at 0.266 bytes/sample.

The candidate uses the accepted clamped-gradient analysis as a deterministic
content classifier. At lossy gray10 only, a frame whose baseline entropy
payload is greater than 0.30 bytes/sample is re-encoded with MED and signaled
with bit 6 of the depth byte. All other gray frames retain clamped gradient;
gray16 refinement, nongray MED, steps, entropy coding, and tile geometry are
unchanged.

The falsifiable hypothesis is that high-rate gray10 content benefits from
MED's edge clamping: canonical rejection will strictly reduce the controlling
generation-quality violation, introduce no new failure or regression, and not
increase total bytes. The selector should keep the lower-rate known
`raw-40-gray-10` regression on its accepted reconstruction path in full.

## Canonical command and artifacts

```sh
PYTHONPATH=.:cuda python3 scripts/evaluate.py --tier rejection \
  --output <source-keyed-artifact> --libvship-revision v5.0.0 \
  --libvship-build '5.0.0 CUDA direct C API' \
  --libvship-gpu-id 0 --libvship-workers 2
```

Cached baseline:

- `evaluation_results/rejection-c249ce4bc02dee2eab5f2972ce80b764ac1fd22e634059f74bb6f76ac6e5f1ce.json`.

Candidate source SHA-256:
`b84325a88279126efd39ab3fae873e5698e32e35b2f446495d027e35ca31708b`.
Candidate artifact:

- `evaluation_results/rejection-b84325a88279126efd39ab3fae873e5698e32e35b2f446495d027e35ca31708b.json`.

## Result

The focused CUDA suite passed all 14 tests.

| Codec | Bytes | Ratio | Ordinary min/max | Generation min/max | Failures |
|---|---:|---:|---:|---:|---:|
| baseline | 322,248,140 | 6.679315x | 94.813339 / 0.747622 | 89.081276 / 2.482678 | 5 |
| candidate | 322,199,847 | 6.680316x | 94.813339 / 0.747542 | 89.327042 / 2.308000 | 5 |

The candidate saves 48,293 bytes, strictly improves both controlling
generation extrema, and introduces no new failure identity. On
`ai-13-gray-10`, MED saves 48,293 bytes and improves generation quality from
89.081276 / 2.482678 to 89.327042 / 2.308000. Its encode median increases
from 0.6463 ms to 1.0907 ms but remains within the canonical gate; decode
increases from 0.3750 ms to 0.4294 ms and also passes. Candidate rejection
artifact SHA-256 is
`00289eca66773ea022b3d223dab0585e38f30390cb820f38f21ad52119a46540`.

The unchanged candidate advanced to the full tier using cached baseline
`evaluation_results/full-c249ce4bc02dee2eab5f2972ce80b764ac1fd22e634059f74bb6f76ac6e5f1ce.json`
and candidate
`evaluation_results/full-b84325a88279126efd39ab3fae873e5698e32e35b2f446495d027e35ca31708b.json`.

| Codec | Bytes | Ratio | Ordinary min/max | Generation min/max | Failures |
|---|---:|---:|---:|---:|---:|
| baseline | 2,123,884,240 | 6.443741x | 88.391052 / 1.632229 | 81.743011 / 4.645103 | 173 |
| candidate | 2,121,516,178 | 6.450934x | 88.391052 / 1.632229 | 81.743011 / 4.645103 | 173 |

The candidate saved 2,368,062 bytes (0.112%) and changed 18 high-rate gray10
samples. It introduced no new failure identity, removed none, and produced no
correctness, coverage, determinism, or performance failure. Full artifact
SHA-256 is
`8d3b073dab71d506c788adc724858373cf2e01fe8cc7ac69ab979ef3ee925629`.

## Conclusion

Reject and restore accepted EXP-0193. Although the selector isolated the known
lower-rate `raw-40-gray-10` regression, saved materially more bytes in full
than in rejection, and improved rejection's controlling `ai-13-gray-10`
violation, the full-corpus worst ordinary and generation extrema were exactly
unchanged. It therefore fails the failing-baseline exception's requirement to
strictly reduce the worst-case quality violation. The 0.30-byte/sample gate
also selected 17 other gray10 samples with mixed quality movement, so it is
not a general quality-safe classifier.

Related: [EXP-0182](EXP-0182-median-edge-predictor.md),
[EXP-0184](EXP-0184-med-extrema-repair.md), and
[EXP-0193](EXP-0193-rate-gated-gray16-repair.md).
