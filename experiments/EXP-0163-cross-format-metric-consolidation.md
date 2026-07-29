# EXP-0163 — Cross-format metric consolidation

Status: **ACCEPTED**

Date: 2026-07-29

## Hypothesis

Because gray, YUV422, and RGB444 inputs already converge to the same
native-depth YUV444 metric interchange, losslessly concatenating format-specific
segments will reduce FFVShip invocations without changing per-frame scores.

## Method

Group samples by resolution and bit depth, yielding six compatibility groups.
Within each group, retain one raw segment per source format and convert it through
the unchanged full-range BT.709, native-depth YUV444 FFV1 path. Stream-copy the
segments into one Matroska sequence, invoke SSIMULACRA2 and Butteraugli once each,
and map scores back in segment/frame order. Codec correctness and performance
remain evaluated per sample. Process one group at a time in `/dev/shm`.

Canonical command based on revision `f1f0f38` plus the methodology candidate:

```sh
PYTHONPATH=. python3 scripts/evaluate.py --tier rejection \
  --output /tmp/fastvid-v7-six-group-rejection-rerun.json \
  --ffvship-revision v5.0.0 \
  --ffvship-build '5.0.0-a CUDA, explicit gpu-id 0' \
  --ffvship-gpu-id 0 --quality-temp /dev/shm
```

## Results

| Method | Groups | FFVShip calls | Convert/concat | Metrics | Wall | Result |
|---|---:|---:|---:|---:|---:|---|
| source-format groups | 9 | 18 | 48.422 s | 133.061 s | 210.061 s | pass |
| resolution/depth, run 1 | 6 | 12 | 61.887 s | 111.230 s | 204.495 s | performance fail |
| resolution/depth, run 2 | 6 | 12 | 65.277 s | 113.177 s | 211.897 s | pass |

Both candidate runs reproduced minimum SSIMULACRA2 `94.112907409668`, maximum
Butteraugli `0.985730707645416`, and compression ratio `6.188000859134071`
exactly. Thirteen focused evaluator/extraction tests passed, including actual
FFmpeg concatenation and frame-count validation.

Cross-format consolidation saves roughly 20 seconds of FFVShip time but adds
roughly 17 seconds of conversion/concatenation on this small rejection set. Total
wall time is therefore neutral relative to normal run variance. Its principal
benefit is reducing process launches and producing longer sequences; the full
corpus should amortize fixed segment work more effectively. Peak RSS rose from
12.1 GiB to about 21.4 GiB, within the available 88 GiB tmpfs budget.

The first run failed the 1 ms encode-latency gate: the designated 1080p RGB10
case measured 1.040 ms and an ordinary RGB16 quality sample measured 1.068 ms.
The idle-GPU rerun measured 0.931 ms for the designated case and passed. This
exposes meaningful near-threshold performance variance but no quality or
compression difference; both artifacts are retained.

Artifacts:

- `/tmp/fastvid-v7-six-group-rejection.json`;
- `/tmp/fastvid-v7-six-group-rejection-rerun.json`.

## Decision

Accept grouping by resolution and bit depth. It preserves every per-frame result
and all matrix coverage while cutting FFVShip calls by one third. Treat the
observed latency variance separately; do not use the failed first run as an
acceptance result. Evaluate corpus stratification as a distinct methodology
experiment rather than combining it with this call-consolidation change.
