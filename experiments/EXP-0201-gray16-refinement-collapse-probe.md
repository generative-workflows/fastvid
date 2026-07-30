# EXP-0201 — Gray16 refinement collapse probe

Status: **REJECTED**

Date: 2026-07-30

Baseline revision: `4b8a9d1` (accepted EXP-0200).
Baseline codec-source SHA-256:
`a694cd12c51b445edb6f6e33e5f2b7f4a0611aa23d53e66301a59ee150d78b74`.
Evaluator revision: `ed4febd5b9b2815fc86e1f36e50ef4a63b8eac46`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

## Hypothesis and rationale

EXP-0200 stabilized `procedural-02-gray-16` by widening the gray16 baseline
payload gate from 0.10 to 0.20 byte/sample, raising its generation SSIMULACRA2
from 81.743011 to 87.259956. The broader gate also refined natural samples
near 0.19 byte/sample. In particular, `raw-47-gray-16` became the new full
generation controller: its ordinary quality improved, but generation SSIM fell
from 89.148621 to 85.083862 and its source grew by 265,490 bytes.

Canonical per-cycle artifacts expose a deterministic separation. Unstable
`procedural-02` frames encode near 0.18 byte/sample at the baseline step but
collapse below 0.04 byte/sample when probed at step 321. The affected natural
samples instead expand above 0.20 byte/sample at step 321. The candidate keeps
the accepted direct arms below 0.10 and above 0.50 byte/sample, but treats the
0.10--0.20 band as a probe: retain step 321 only if its exact complete entropy
payload falls below 0.10 byte/sample; otherwise rerun and emit the baseline
step. The bitstream flag continues to signal the final choice.

Gray10 MED funding and all other accepted EXP-0200 behavior remain unchanged.
The falsifiable hypothesis is that rejection remains eligible through the
unchanged `ai-13-gray-10` improvement, while full restores natural mid-rate
gray16 reconstruction, preserves the procedural stability repair, strictly
improves the 85.083862 worst generation SSIM, reduces bytes, adds no failures,
and passes all performance gates despite the bounded probe pass.

## Canonical command and artifacts

```sh
PYTHONPATH=.:cuda python3 scripts/evaluate.py --tier <rejection|full> \
  --output <source-keyed-artifact> --libvship-revision v5.0.0 \
  --libvship-build '5.0.0 CUDA direct C API' \
  --libvship-gpu-id 0 --libvship-workers 2
```

- rejection baseline cache hit:
  `evaluation_results/rejection-a694cd12c51b445edb6f6e33e5f2b7f4a0611aa23d53e66301a59ee150d78b74.json`;
- full baseline cache hit if required:
  `evaluation_results/full-a694cd12c51b445edb6f6e33e5f2b7f4a0611aa23d53e66301a59ee150d78b74.json`.

Candidate codec-source SHA-256:
`21cb1a56e5538420b5d6c91b174b71af996e60a10297043341c351225abd8d2f`.

- candidate rejection:
  `evaluation_results/rejection-21cb1a56e5538420b5d6c91b174b71af996e60a10297043341c351225abd8d2f.json`.

## Result

The focused CUDA suite passed all 14 tests. The codec output on the rejection
subset was byte-for-byte and metric-for-metric identical to accepted EXP-0200:
322,199,847 bytes at 6.680316x, ordinary extrema 94.813339 / 0.747542,
generation extrema 89.327042 / 2.308000, and the same five quality failures.

However, the canonical candidate artifact introduced
`performance-1080p-rgb444-10: decode latency >= 0.5 ms`. Decode median was
0.510400 ms versus the cached baseline's 0.480144 ms. The source modification
is confined to gray16 encoder selection and cannot change RGB10 decode work,
but the recorded performance gate is binding. Candidate rejection artifact
SHA-256 is
`e9f8e1ee7cfb46ecd7350fa5e274921d3775de21d6d400e534ff5de435bcc517`.

## Conclusion

Reject at rejection and restore accepted EXP-0200. Any new performance failure
requires rejection, so the unchanged candidate cannot proceed to full and the
collapse hypothesis remains untested at its intended scope. A successor may
combine the same encoder selector with an attributable decoder-headroom change
only if that change is independently justified; the source-keyed failed
artifact must remain cached and must not be overwritten or rerun.

Related: [EXP-0193](EXP-0193-latency-hardened-gray-repair.md) and
[EXP-0200](EXP-0200-med-funded-stable-gray16.md).
