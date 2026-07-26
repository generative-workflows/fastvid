# EXP-0123 — Matched direct-emission isolation

Status: **ACCEPTED**

## Classification

**Version-5 frontier resolution / evaluation exploitation** — determine
whether EXP-0120's 10.07% encode gain survives matched complete binaries and
whether its unchanged-decoder regression is reproducible in isolation.

## Hypothesis

Rebuilding both EXP-0117 and direct-emission variants with the identical
EXP-0122 decode-only command should measure source-identical isolated decode
within 5%. A fresh balanced whole-codec comparison should retain at least
1.05x geometric encode. Direct emission may be revived only if the fresh
whole-codec decode result also retains at least 0.95x; isolated decode is
diagnostic and does not override the promotion gate.

## Modification

Add a balanced arbitrary-binary wrapper for
`benchmark-decode16`. Warm both binaries, alternate execution order over five
trials, preserve every row, validate invariant input fields, and report the
median candidate/reference decode ratio.

Build the current paired-selector source and the exact EXP-0120 direct
count/scan/disjoint-write modification from the same tree containing the
isolated command. Do not change syntax, decoder source, or benchmark controls.

## Test

- verify separate binary hashes and the accepted version-5 control hash;
- run five balanced isolated-decode trials with repeated warm-cache work;
- require at least 0.95x isolated decode throughput and invariant encoded
  bytes, bit depth, and luma area;
- run five balanced whole-codec trials across the native high-bit manifest;
- require at least 1.05x encode and 0.95x decode geometrically with identical
  bytes, bitrate, PSNR, SSIM, and maximum error;
- accept direct emission only if both the ordinary and diagnostic gates pass.

## Result

The matched paired-selector binary has SHA-256
`a1ccd598dabfdb20b86ac1752d7c2ba8f7961e66bc9435f8a47c2ca8fb3eb441`;
the matched direct-emission binary has SHA-256
`d828b8a79f94194baa3f1593a9acf67a6a4f915dd443b8d7120fc088c06291dc`.
Both include the identical isolated-decode command. The candidate retains the
accepted control SHA-256
`9a3cf708ecdc73f9f8c15a545b41f761ad1ed844c2b8cb4db42118ce587fce37`.

Five balanced isolated-decode trials, each performing 20 warm-cache decodes
of that 1920x1080 10-bit frame, measured:

| Variant | Median decode |
|---|---:|
| paired selector | 59.581 MP/s |
| direct emission | 59.492 MP/s |
| **Candidate/reference** | **0.9985x** |

Encoded bytes, threads, repetitions, bit depth, and luma area are invariant.
The isolated 0.95x gate passes with only a 0.15% movement.

Five balanced whole-codec trials across the native high-bit manifest measured:

| Sample | Candidate encode | Encode ratio | Decode ratio |
|---|---:|---:|---:|
| HDR gradient 10 | 33.182 MP/s | 1.116x | 0.983x |
| Precision motion 10 | 34.083 MP/s | 1.118x | 0.984x |
| Precision UI 12 | 35.355 MP/s | 1.056x | 0.987x |
| Precision motion 16 | 49.057 MP/s | 1.161x | 1.029x |
| **Geometric** | — | **1.1121x** | **0.9956x** |

Both ordinary 1.05x encode and 0.95x decode gates pass. Every encoded byte,
compression ratio, bitrate, PSNR component, luma block SSIM, and maximum
error remains identical. Relative to EXP-0110's fixed version-2 rows,
version-5 encode advances from approximately 0.4742x to 0.5274x without
moving its rate, quality, decode, or access point.

Artifacts:

- `artifacts/exp0123-isolated-decode.tsv`
  (`e7dd64f1681d64e253989c93e0188561ca26b4c82c13f2b6e4e039c1da1f2752`);
- `artifacts/exp0123-whole-codec.tsv`
  (`7bec1d0c35b1426269a0e5d25fc8b7a3b2e784395fafbc0fdc84dc1d329420a1`).

All 68 library tests, including the direct-lane/vector-reference oracle,
motion/squeeze tests, binary and documentation tests, normal and
profiling-feature strict Clippy, shell syntax, formatting, and diff checks
pass.

## Decision

Accept direct final-lane emission and supersede EXP-0120's implementation
rejection with matched evidence. The earlier complete-binary decode
regression was not reproduced after both variants were rebuilt from the same
harness-bearing tree; isolated decode independently confirms no algorithmic
decoder cost.

The production encoder now reuses exact selector lengths, allocates each Rice
body once, and writes four lanes directly into scan-assigned disjoint spans.
This removes lane-to-body payload copies and is the CPU form of the intended
CUDA count/scan/disjoint-write contract.

Keep version 5 experimental: approximately 33–49 MP/s remains below the
preserved version-2/OpenAPV speed target. Profile the accepted writer before
choosing between predictor/residual staging, remaining Rice selection, and
zero-run construction.

## References

- [EXP-0117](EXP-0117-paired-rice-parameter-pass.md)
- [EXP-0120](EXP-0120-direct-rice-lane-emission.md)
- [EXP-0121](EXP-0121-emission-binary-frontend-counters.md)
- [EXP-0122](EXP-0122-isolated-decode-harness.md)
