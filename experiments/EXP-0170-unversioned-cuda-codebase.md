# EXP-0170 — Unversioned CUDA codebase

Status: **ACCEPTED**

Date: 2026-07-29

## Problem

The only live CUDA implementation used `encode_v5` and `decode_v5` names even
though it emitted header version 7 and retained decode branches for experimental
versions 5 and 6. Benchmarks, tests, source filenames, error messages, and docs
repeated those historical labels. The Rust implementation referenced by the old
CUDA tests had already been removed.

Fastvid is still a pre-v1 research codebase. Experiments produce the next v1
candidate; they do not create supported implementation or compatibility lines.

## Change

Collapse the live implementation to one unversioned path:

- rename `encode_v5.cu` and `decode_v5.cu` to `encode.cu` and `decode.cu`;
- expose only `encode` and `decode` from C++, `fastvid_cuda`, and `fastvid`;
- rename live tests, benchmarks, profiles, kernels, and helper symbols;
- remove the version-5 and version-6 quantizer/decode branches;
- emit and accept only header format discriminator 1;
- remove the extension distribution's independent release number;
- replace the missing Rust-oracle tests with direct CUDA roundtrip,
  determinism, entropy-mode, CPU/VRAM-input, predictor, and malformed-stream
  coverage.

Historical experiment and research records retain their original terminology.
They are evidence, not live compatibility surfaces.

## Compatibility

Streams emitted by experimental implementations 5, 6, and 7 are intentionally
unsupported. There is no released bitstream compatibility promise to preserve.
The 32-byte header retains discriminator 1 so malformed or stale experimental
streams are rejected rather than silently decoded with the current quantizer.

## Validation

The extension rebuilt successfully with CUDA 12.8. The combined live codec,
evaluator, and corpus suite passed:

```text
31 passed
```

The canonical rejection tier passed all 11 samples:

- minimum SSIMULACRA2: `93.69731903076172`;
- maximum Butteraugli: `0.08440515398979187`;
- compression ratio: `6.188000859134071`;
- no correctness, coverage, quality, or performance failures.

Artifact: `/tmp/fastvid-unversioned-v1-rejection.json`.

## Decision

Accept the unversioned live CUDA codebase and single pre-v1 stream format.
Future experiments modify these files and APIs in place. Do not add numbered
implementation filenames, numbered public functions, legacy decoders, or
parallel version branches before an actual compatibility commitment exists.
