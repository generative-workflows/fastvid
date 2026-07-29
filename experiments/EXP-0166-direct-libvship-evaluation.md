# EXP-0166 — Direct in-memory libvship evaluation

Status: **ACCEPTED**

Date: 2026-07-29

## Hypothesis

The evaluator already holds canonical source planes and fastvid decoded planes
in memory. Passing those planes directly to libvship will remove all metric
media staging and make quality evaluation reflect the intended raw-roundtrip
comparison.

## Methodology

Replace FFVShip subprocesses, FFmpeg conversion, FFV1 intermediates, Matroska
containers, and FFMS2 decoding with libvship v5.0.0's C API loaded through a
small ctypes bridge. The bridge:

- copies source and decoded CUDA tensors into pinned native-depth host
  tensors without writing files;
- passes native full-range BT.709 YUV422 planes directly with left chroma
  location;
- passes native RGB planes directly with BT.709 transfer and primaries;
- supplies the one gray plane as R, G, and B without changing its samples;
- narrows declared 8-bit uint16 corpus containers to uint8 and preserves uint16
  for 10-bit and 16-bit inputs;
- keeps two independent handlers per metric and runs SSIMULACRA2 and
  Butteraugli concurrently;
- uses Butteraugli q=2 and retains the conservative maximum of q=2, q=3, and
  infinity norms for gating.

Handler pools persist across samples of one resolution/format/depth and are
freed before moving to the next format. Codec timing remains unchanged and
excludes all metric copies and computation. Reports use schema version 3 and
record the libvship path, revision, build, GPU id, worker count, interface, and
colorspace policy.

## Controls

An isolated 1080p RGB10 corpus-frame prototype returned zero identical-input
Butteraugli and a positive fastvid-roundtrip distance. The permanent test suite
uses a 256x256 RGB10 CUDA control and requires:

- identical Butteraugli exactly zero;
- identical SSIMULACRA2 above the project gate;
- a deterministic plane perturbation to lower SSIMULACRA2 and increase
  Butteraugli.

Unit tests also assert native YUV422 subsampling, RGB/gray family selection,
full range, and exact 8/10/16-bit libvship sample types. The canonical rejection
tier then exercises every required format/depth cell, including multi-frame
batches. Fifteen focused evaluator/extraction tests pass.

## Canonical command

```sh
PYTHONPATH=. python3 scripts/evaluate.py --tier rejection \
  --output /tmp/fastvid-v7-direct-libvship-rejection.json \
  --libvship-revision v5.0.0 \
  --libvship-build '5.0.0 CUDA direct C API' \
  --libvship-gpu-id 0 --libvship-workers 2
```

## Results

| Method | Metric work | Report elapsed | External wall | Peak RSS | Result |
|---|---:|---:|---:|---:|---|
| FFVShip, initial workers | 133.061 s | 210.061 s | ~211 s | ~12.1 GiB | pass |
| FFVShip, tuned workers | 58.993 s | 148.239 s | 148.87 s | ~10.9 GiB | pass |
| Direct libvship | 1.530 s | 22.142 s | 24.06 s | 4.40 GiB | pass |

Direct evaluation is 6.2 times faster by external wall time than tuned FFVShip
and about 8.8 times faster than the initial consolidated evaluator. It removes all
transient metric storage. The 57-frame rejection tier sustains 37.2 aggregate
direct metric frames/s across mixed resolutions, formats, and depths.

The direct native-plane methodology intentionally establishes a new quality
baseline rather than reproducing the previous FFmpeg/YUV444 scores:

- minimum SSIMULACRA2: `93.69731903076172`;
- maximum Butteraugli: `0.08440515398979187`;
- compression ratio: `6.188000859134071`;
- all correctness, quality, coverage, and codec performance gates pass.

Artifact: `/tmp/fastvid-v7-direct-libvship-rejection.json`.

## Decision

Accept direct in-memory libvship as the canonical quality interface. The project
introduced FFVShip without a pre-existing metric baseline, so no compatibility
constraint requires retaining its container and conversion behavior. Future
codec candidates must compare against this direct native-plane baseline and
must not mix scores from the retired FFVShip/YUV444 methodology.
