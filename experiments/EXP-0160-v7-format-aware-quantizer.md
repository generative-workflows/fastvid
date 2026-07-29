# EXP-0160 — Version-7 format-aware quantizer

Status: **PENDING FULL CORPUS**

Implementation revision: `f5de3899dd287e82aa7dc82469f76aeb4d515db9`

## Hypothesis

The canonical corpus failures are concentrated in gray8 and YUV, while a
global q98 step breaks RGB latency. Format/depth-aware slopes can clear every
q90 rejection quality gate without paying the global q98 rate and speed cost.

## Modification

Version new streams as Fastvid v7. Preserve v5 and v6 decoder semantics. For
v7, derive `step = 1 + ceil((100-quality)*2^(depth-8)/denominator)` using:

- gray8: step 1 at every quality;
- YUV422 10-bit: denominator 20;
- YUV422 16-bit: denominator 12;
- RGB444 10-bit: denominator 10;
- every other cell: the v6 denominator 6.

This selects q90 steps 1/3/215/5 only where the canonical boundary requires
them, retaining coarser v6 steps in already-passing cells. Prediction, entropy
coding, and `scripts/evaluate.py` are unchanged.

## Canonical evaluation

All runs use the checked extracted corpus revision
`fastvid-corpus-v1-extracted-1`, FFVShip 5.0.0-a, five warm-ups, and 20 timed
repetitions.

- failing v6 q90 control: `/tmp/fastvid-corpus-v1-rejection11-q90.json`;
- failing v6 q95 boundary: `/tmp/fastvid-corpus-v1-rejection11-q95.json`;
- failing global q98 boundary: `/tmp/fastvid-corpus-v1-rejection11-q98.json`;
- passing v7 q90 candidate: `/tmp/fastvid-v7-format-aware-q90.json`.

## Rejection result

| Candidate | Pass | Encoded bytes | Ratio | Min SSIMU2 | Max Butteraugli |
|---|---:|---:|---:|---:|---:|
| v6 q90 | no | 289,102,984 | 7.4451x | 81.8132 | 2.4743 |
| v6 q95 | no | 325,364,409 | 6.6153x | 89.1268 | 1.7315 |
| v6 q98 | no | 401,887,746 | 5.3557x | 93.3239 | 1.1454 |
| v7 q90 | yes | 347,833,953 | 6.1880x | 94.1129 | 0.9857 |

V7 is 13.45% smaller than global q98. Its required performance cases pass:
YUV422 encode/decode 3.375/7.152 GP/s, RGB444 2.587/5.316 GP/s, and 1080p
RGB444 encode/decode 0.922/0.450 ms. Q100 is exact in all eight required cells;
a same-step v6 stream decodes identically and version 8 is rejected.

## Decision

Retain pending the unchanged full tier. Rejection establishes feasibility but
cannot accept the candidate or establish corpus-wide compression.

## References

- [EXP-0159](EXP-0159-v6-sixth-scale-quantizer.md)
- [EXP-0151](EXP-0151-corpus-v4-full-frame-feedback.md)
- [Research 0014](../research/0014-sampling-and-high-bit-quantization.md)
