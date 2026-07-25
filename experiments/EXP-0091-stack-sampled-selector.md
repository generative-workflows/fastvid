# EXP-0091 — Stack-backed sampled selector

Status: **REJECTED**

## Classification

**Allocation exploitation** — remove one heap allocation from every
high-bit speed-tier tile after EXP-0090 attributed 42.82% of encode-only
cycles to the fused tile closure.

## Hypothesis

The sampled entropy selector allocates a `Vec<u32>` for one source row on
every tile. Active 256x128 geometry needs at most 256 sampled values, so a
1 KiB stack buffer with a general heap fallback for wider tiles should
improve matched q90 encode by at least 2% without changing selection, bytes,
or quality.

## Modification

1. Fill a `[u32; 256]` stack buffer when tile width is at most 256.
2. Retain the existing exactly sized `Vec` fallback for arbitrary wider
   tiles.
3. Pass the resulting slice to the unchanged zero-run/Rice/block-pack cost
   models.
4. Do not change tile geometry, sampled row, predictor, entropy mode, or
   syntax.

This is a storage optimization, not a 256-wide format assumption.

## Gate

- byte- and metric-identical q90/q100 focused streams;
- at least 2% matched q90 one-thread encode improvement;
- decode no worse than 5%;
- widths above 256 exercise the heap fallback in a selector-equivalence test;
- strict Clippy, formatting, and relevant release tests pass; and
- no slow-tier run unless the focused gate passes.

## Result

The boundary test compared the stack path at widths 255 and 256 and the
heap fallback at widths 257 and 300 with an allocation-based reference.
Every selector result matched. Strict release Clippy and formatting passed.

A balanced two-trial focused screen then measured the two native
high-precision motion samples at q90, one thread:

| Depth | Baseline encode | Candidate encode | Delta | Decode delta | Bytes |
|---:|---:|---:|---:|---:|---:|
| 10-bit | 68.739 MP/s | 68.543 MP/s | -0.286% | +0.607% | identical |
| 16-bit | 67.413 MP/s | 68.702 MP/s | +1.912% | +0.305% | identical |
| geometric aggregate | 68.073 MP/s | 68.622 MP/s | +0.807% | +0.456% | identical |

PSNR and block SSIM were also identical. The result is well below the
declared 2% encode gate, and the matched 10-bit sample—the current OpenAPV
comparison path—slightly regressed. Per the fast-feedback policy, no
six-trial or slow-tier run was performed.

Artifacts:

- focused raw results:
  `artifacts/exp0091-stack-sample-smoke.tsv`
  (`5a1d9f4f99b9682d8f75804f4386ff571bd52f30aec4770da16998d23375d8a7`);
- candidate binary:
  `artifacts/frontier/fastvid-speed-exp0091-stack-sample`
  (`f1122a75ff19b25fa5ad79634980ce5c804ce517103430bb5bb7363647f0d438`).

## Decision

Reject the stack buffer and restore EXP-0088 unchanged. One allocation per
tile in this selector is not a large enough share of practical-q90 encode
time to pursue. The differing 10-bit and 16-bit directions also make the
small aggregate gain unsafe to generalize.

## References

- [Research 0012](../research/0012-simd-cache-profiling.md)
- [EXP-0077](EXP-0077-high-bit-prefix-rice-streaming.md)
- [EXP-0090](EXP-0090-post-pack-speed-profile.md)
