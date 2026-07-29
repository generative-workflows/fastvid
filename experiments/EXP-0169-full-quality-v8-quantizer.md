# EXP-0169 — Full-quality v8 quantizer

Status: **REJECTED**

Date: 2026-07-29

Implementation revision: `01070af43599fddf77a9f131cce3fe389c585dc8`

## Hypothesis

The stratified full-tier failures are confined to YUV422-8 and one sample in
each of gray-10, gray-16, and RGB444-16. Making YUV422-8 lossless and halving
the quantizer step in the three high-depth cells will clear those failures
while preserving enough entropy efficiency to improve rejection-tier
compression over v7.

## Modification

Version encoder output as Fastvid v8 while retaining v5, v6, and v7 decoder
semantics. At q90, v8 uses step 1 for YUV422-8 and denominator 12 for gray-10,
gray-16, and RGB444-16. Other cells retain the v7 mapping. This is one
attributable format/depth quantizer-map change; prediction, entropy coding,
evaluator logic, and corpus are unchanged.

## Canonical commands

Both runs used:

```sh
PYTHONPATH=. python3 scripts/evaluate.py --tier rejection \
  --output ARTIFACT.json --libvship-revision v5.0.0 \
  --libvship-build '5.0.0 CUDA direct C API' \
  --libvship-gpu-id 0 --libvship-workers 2
```

Corpus revision: `fastvid-corpus-v1-extracted-2`.
Evaluator baseline revision: `a12f899`.

Artifacts:

- baseline: `/tmp/fastvid-exp0169-baseline-rejection.json`;
- candidate: `/tmp/fastvid-exp0169-candidate-v8-rejection.json`.

An earlier `/tmp/fastvid-exp0169-candidate-rejection.json` is invalid: the
prototype emitted a v7 tag with v8 quantization and was corrected before the
reported candidate run.

## Result

| Revision | Pass | Ratio | Min SSIMULACRA2 | Max Butteraugli |
|---|---:|---:|---:|---:|
| v7 baseline | yes | 6.1880008591x | 93.697319 | 0.084405 |
| v8 candidate | yes | 6.0956317515x | 91.115219 | 0.119811 |

All correctness, coverage, quality, and performance gates passed in the final
candidate run. The candidate nevertheless encoded 1.515% more bytes than the
baseline. Twelve focused tests passed; four Rust-oracle CUDA tests could not
start because `target/release/fastvid` is absent. Canonical evaluation itself
completed successfully.
