# EXP-0162 — Consolidated FFVShip sequences

Status: **ACCEPTED**

Date: 2026-07-29

## Hypothesis

FFVShip already emits one score per frame. Concatenating compatible samples into
one sequence will preserve every per-frame gate while amortizing process startup,
FFMS2 indexing, and decoder initialization.

## Method

Restore the validated FFVShip v5.0.0-a CUDA build as `/usr/local/bin/FFVship`,
with a version-specific `libvship.so` rpath. Preserve the rejected 5.1.1-a binary
as `/usr/local/bin/FFVship-5.1.1-a`. Group corpus entries by width, height, source
format, and bit depth. Codec correctness and CUDA performance remain serialized
and per sample. Append reference and decoded frames in manifest order, convert
each consolidated raw sequence to native-depth YUV444 FFV1, run SSIMULACRA2 and
Butteraugli concurrently once per group, then map their frame scores back to each
sample. Process and delete one group at a time.

Canonical command at the working-tree candidate based on codec revision
`70237ed`:

```sh
PYTHONPATH=. python3 scripts/evaluate.py --tier rejection \
  --output /tmp/fastvid-v7-consolidated-rejection.json \
  --ffvship-revision v5.0.0 \
  --ffvship-build '5.0.0-a CUDA, explicit gpu-id 0' \
  --ffvship-gpu-id 0 --quality-temp /dev/shm
```

## Results

| Evaluator | FFVShip calls | Wall time | Min SSIMU2 | Max Butteraugli | Result |
|---|---:|---:|---:|---:|---|
| per-sample baseline | 22 | 238.699 s | 94.1129 | 0.9857 | pass |
| consolidated | 18 | 210.061 s | 94.1129 | 0.9857 | pass |

The compact rejection set contains 57 frames in 11 samples but only two pairs
share a compatibility group, producing nine groups. Wall time fell by 28.638 s
(12.0%; 1.14x throughput). Across groups, FFmpeg conversion consumed 48.422 s
and the two concurrently launched FFVShip metrics consumed 133.061 s. This is
0.428 evaluated frames/s across heterogeneous groups, while the complete evaluator achieved 0.271 frames/s
over all 57 frames and the 210.061 s end-to-end run. Peak evaluator
RSS was 12.07 GiB. The two useful long groups measured 0.788 metric frames/s for
25-frame 4K YUV422 10-bit and 0.656 metric frames/s for 24-frame 4K RGB444 10-bit.

The metric extrema and compression ratio (`6.188000859`) exactly match the
validated 5.0.0-a baseline to the recorded precision. The report passed all
quality, correctness, coverage, and codec-performance gates. Twelve evaluator
and extraction regression tests passed.

Artifact: `/tmp/fastvid-v7-consolidated-rejection.json`.

## Decision

Accept consolidated metric calls as evaluation methodology. The gain is modest
on the intentionally tiny rejection set but should be much larger on the full
corpus, where many samples share each compatibility group. Reports use schema 2
and include per-group conversion seconds, metric seconds, and metric frame rate.
