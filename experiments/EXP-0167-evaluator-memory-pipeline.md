# EXP-0167 — Evaluator memory-pipeline accounting

Status: **ACCEPTED WITH PARTIAL REJECTION**

Date: 2026-07-29

## Hypothesis

The direct-libvship evaluator can shorten feedback by retaining canonical
pinned host planes, reusing metric buffers, and avoiding a second source-file
hash pass. Explicit phase accounting will identify which changes actually help.

## Methodology

Starting from EXP-0166, add per-sample and aggregate wall-time accounting for
corpus load/validation/upload, correctness and stream setup, CUDA timing,
final decode validation, metric transfers, and metric computation. Return the
SHA-256 values computed during input validation instead of reopening and
rehashing every source when assembling the report. Cache compatible pinned
source and decoded metric buffers for the lifetime of each persistent libvship
pool.

A first variant also copied every canonical plane into retained pinned host
memory before uploading it to CUDA. That variant was tested and then removed:
it duplicated large batches in pinned memory and made the rejection run slower.
The accepted implementation retains the ordinary CUDA corpus representation and
reuses pinned buffers only at the metric boundary.

Codec CUDA-event timing and all correctness, quality, compression, coverage,
and performance gates are unchanged.

## Validation

```sh
PYTHONPATH=. pytest -q tests/test_evaluate.py tests/test_extract_corpus.py

PYTHONPATH=. python3 scripts/evaluate.py --tier rejection \
  --output /tmp/fastvid-v7-direct-pipeline-confirmation-rejection.json \
  --libvship-revision v5.0.0 \
  --libvship-build '5.0.0 CUDA direct C API' \
  --libvship-gpu-id 0 --libvship-workers 2
```

The focused suite passes: 15 tests.

## Results

The retained-canonical-pinned variant took 24.492 s of report time and failed
two 1.0 ms encode gates due to run-to-run timing noise. Its externally measured
wall time was 26.56 s and peak RSS was 4.81 GiB. It was rejected.

The final confirmation run passed all 11 samples:

- report elapsed: 22.045 s, versus 22.142 s for EXP-0166;
- load/validate/upload: 8.800 s;
- correctness and stream setup: 0.287 s;
- CUDA timing wall: 5.893 s;
- final decode validation: 0.068 s;
- metric transfer: 2.948 s;
- metric computation: 1.747 s;
- minimum SSIMULACRA2: `93.69731903076172`, unchanged;
- maximum Butteraugli: `0.08440515398979187`, unchanged;
- compression ratio: `6.188000859134071`, unchanged.

A preceding exact-code run failed two 0.5 ms decode gates and the confirmation
passed them, demonstrating that sub-millisecond single-run gates can flap under
host/GPU scheduling noise. The gates were not weakened.

Artifact: `/tmp/fastvid-v7-direct-pipeline-confirmation-rejection.json`.

## Decision

Accept phase accounting, reuse of compatible metric pinned buffers, and reuse
of hashes already computed while loading. Reject retaining a pinned canonical
copy. The measurement shows that the next meaningful pipeline targets are the
8.8 s load/validation/upload phase and the 2.95 s device-to-host metric transfer,
not libvship compute alone. Treat near-threshold performance failures as results
to investigate; do not retry candidates until they pass or relax gates without
a separate methodology experiment.
