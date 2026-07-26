# EXP-0126 — Selector-fused zero-run cost

Status: **ACCEPTED**

## Classification

**Version-5 entropy exploration/exploitation** — reduce the 17.24% zero-run
construction stage without repeating EXP-0111's separate cost scan.

## Hypothesis

EXP-0111 failed because winner-only emission added independent zero-run and
fixed-block scans. The accepted Rice selector already traverses every folded
symbol for parameter zero. Charging exact zero-run bytes during that existing
traversal should add arithmetic but no memory pass, allowing production to
skip zero-run body construction whenever Rice or fixed block is strictly
smaller.

Version-5 layout evidence shows zero-run wins roughly one quarter of shards.
The fused candidate should improve geometric encode throughput by at least
1.05x over EXP-0123, retain at least 0.95x decode throughput, and preserve
canonical tie order and every stream byte.

## Modification

- accumulate exact zero-run run/literal varint cost during the selector's
  parameter-zero pass;
- retain that cost in the exact Rice selection result;
- emit Rice and fixed block as today, then construct zero-run only when its
  charged length is no larger than both competing bodies;
- preserve zero-run, Rice, fixed-block tie order exactly.

Do not add another folded-residual traversal, change entropy syntax, or alter
prediction/reconstruction.

## Test

- compare fused zero-run cost with actual body length across empty/run/literal
  boundaries and representative 4,096-symbol shards;
- retain the exhaustive Rice oracle and accepted version-5 control hash;
- run five balanced whole-codec trials against the exact EXP-0123 binary;
- require at least 1.05x geometric encode, 0.95x geometric decode, and
  identical bytes/bitrate/quality;
- run the full release suite, both strict Clippy configurations, formatting,
  and diff checks.

## Result

The fused cost matches the actual emitted zero-run body for empty, all-zero,
mixed run/literal, varint-boundary, and representative 4,096-symbol shards.
The candidate retains the accepted control SHA-256
`9a3cf708ecdc73f9f8c15a545b41f761ad1ed844c2b8cb4db42118ce587fce37`
and every native q90 output byte.

Five balanced whole-codec trials measured:

| Sample | Candidate encode | Encode ratio | Decode ratio |
|---|---:|---:|---:|
| HDR gradient 10 | 34.930 MP/s | 1.122x | 1.019x |
| Precision motion 10 | 37.940 MP/s | 1.106x | 0.985x |
| Precision UI 12 | 40.463 MP/s | 1.130x | 1.010x |
| Precision motion 16 | 51.609 MP/s | 1.051x | 0.994x |
| **Geometric** | — | **1.1017x** | **1.0019x** |

The geometric gates pass, and every sample retains at least 0.95x encode.
Compression ratio, encoded-stream bitrate, PSNR components, luma block SSIM,
and maximum error remain identical. Relative to EXP-0110's fixed version-2
rows, version-5 encode advances from approximately 0.5274x to 0.5810x without
moving its format, rate, quality, decode, or access point.

The fixed EXP-0123 binary has SHA-256
`d828b8a79f94194baa3f1593a9acf67a6a4f915dd443b8d7120fc088c06291dc`;
the candidate has SHA-256
`739e68994d7a04c602967f8fee0d09d001821dd2551293c769c8d211e8d67f29`.
Artifacts:

- `artifacts/exp0126-fused-zero-cost-confirm.tsv`
  (`654bb5899f2ad36fd427ba5031972da69ef8c488c5f752e5b401fc498e629db6`);
- `artifacts/exp0126-version5-control.fvid` (the accepted stream hash above).

All 69 library tests, motion/squeeze tests, binary and documentation tests,
normal and profiling-feature strict Clippy, formatting, and diff checks pass.

## Decision

Accept fused zero-run costing. Unlike EXP-0111, it introduces no separate
memory pass and avoids constructing a losing zero-run body on most shards.
The gain is broad across 10/12/16-bit still and motion samples.

Version 5 remains an experimental low-serialization branch because its CPU
encoder is still materially behind version 2/OpenAPV fastest. Re-profile the
accepted path; likely remaining targets are predictor/residual staging,
exact Rice selection, and the still-constructed losing Rice/fixed-block body.

## References

- [EXP-0111](EXP-0111-winner-only-shard-emission.md)
- [EXP-0118](EXP-0118-post-paired-rice-profile.md)
- [EXP-0124](EXP-0124-post-direct-emission-profile.md)
