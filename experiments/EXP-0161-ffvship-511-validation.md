# EXP-0161 — FFVShip 5.1.1-a validation

Status: **REJECTED**

Date: 2026-07-29

## Hypothesis

The updated CUDA FFVShip 5.1.1-a build will materially shorten canonical
quality evaluation while preserving metric controls and scores.

## Test

Run the unchanged 11-entry q90 rejection tier at v7 revision `d909324` with
FFVShip pinned to GPU 0, three GPU threads, and two decoder threads. Compare
against the existing FFVShip 5.0.0-a result. Sample `nvidia-smi dmon` during
active metrics. Run Butteraugli with one generated FFV1 video as both source
and distorted input; all emitted norms must be zero.

The installed 5.1.1-a binary requires the matching FFmpeg 7 libraries through
`LD_LIBRARY_PATH=/opt/ffmpeg-7.1.5/lib`.

## Results

| FFVShip | Wall time | Min SSIMU2 | Max Butteraugli | Result |
|---|---:|---:|---:|---|
| 5.0.0-a | 238.699 s | 94.1129 | 0.9857 | pass |
| 5.1.1-a | 228.151 s | 94.1134 | 2.1693 | fail |

The update is 4.42% faster. During two simultaneous metric processes, sampled
L40S SM utilization remained 0--1% while each FFVShip process consumed about
one CPU core. The update therefore does not provide evidence of effective GPU
utilization.

More importantly, the identical-video Butteraugli control emitted:

```json
[[0.26872119307518, 0.288272619247437, 1.10135722160339]]
```

An isolated build of the repository's `v5.0.0` tag against the same FFmpeg 7
and FFMS2 installation emitted `[[0, 0, 0]]` for the identical input.
SSIMULACRA2 and most nonzero Butteraugli values also changed slightly between
versions, so results must not be mixed within one full run.

Artifacts:

- `/tmp/fastvid-v7-rejection-ffvship-5.1.1.json`;
- `/tmp/ffvship-511-control/butter.json`;
- `/tmp/ffvship-511-control/butter500.json`.

## Decision

Reject FFVShip 5.1.1-a for canonical acceptance until its identical-input
Butteraugli regression is resolved. Continue with an isolated 5.0.0-a build;
do not replace or modify the user's installed 5.1.1-a binary. No evaluator or
codec source changed in this experiment.
