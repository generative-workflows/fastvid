# EXP-0093 — Proven unchecked quantizer lookup

Status: **REJECTED**

## Classification

**Cache/kernel exploitation** — remove the per-sample bounds check retained
inside the profiled fused predictor/reconstruction loop.

## Hypothesis

EXP-0090's annotated encode-only profile attributes 6.84% of local fused-loop
samples to the quantizer-table length comparison immediately before the
dependent load. A minimal unchecked lookup, isolated behind a range proof,
should improve matched q90 one-thread encode by at least 2% without changing
bytes, reconstruction, or quality.

## Modification

1. Put the only unsafe operation in a dedicated `quantizer_lut` module.
2. Assert in debug builds that the table has `2 * max_sample + 1` entries and
   that the residual is in `[-max_sample, max_sample]`.
3. Use the codec invariant that source sample and prediction are both in
   `[0, max_sample]` to prove the release lookup index is in range.
4. Keep table construction, quantization values, predictor, entropy syntax,
   decoder, and every stream decision unchanged.

The crate-level lint remains `deny(unsafe_code)` and only this private module
receives an explicit allowance. This replaces the previous crate-wide
`forbid` solely so the narrowly documented module can be compiled.

## Gate

- exhaustive lookup equality for every residual, quality, and 10/12/16-bit
  depth;
- byte- and metric-identical focused q90/q100 streams;
- at least 2% matched q90 one-thread encode improvement;
- decode no worse than 5%;
- strict Clippy, formatting, and release tests pass; and
- no slow-tier run unless the focused gate passes.

## Result

The isolated lookup compiled under a crate-wide `deny` with only the private
module allowed to use unsafe. Strict release Clippy, formatting, and the
existing exhaustive quantizer test passed. That test covers every residual
at every quality for 10, 12, and 16 bits.

A balanced two-trial q90 one-thread screen measured:

| Depth | Baseline encode | Candidate encode | Delta | Decode delta | Bytes |
|---:|---:|---:|---:|---:|---:|
| 10-bit | 72.129 MP/s | 70.816 MP/s | -1.820% | -1.160% | identical |
| 16-bit | 67.997 MP/s | 68.069 MP/s | +0.106% | +0.383% | identical |
| geometric aggregate | 70.032 MP/s | 69.429 MP/s | -0.861% | -0.391% | identical |

PSNR and block SSIM were identical. The primary matched path regressed
instead of meeting the +2% gate, so no six-trial or slow-tier run was
performed.

Artifacts:

- focused raw results:
  `artifacts/exp0093-unchecked-quantizer-smoke.tsv`
  (`417240721279cf1343d1d5ecc58e4fce359b87480d3236c653a9b12d8e411f61`);
- candidate binary:
  `artifacts/frontier/fastvid-speed-exp0093-unchecked-quantizer`
  (`4353625de44960010334cb6d50ad0637ed144e997d759a1939a2a33a19db7c1e`);
- isolated lookup source:
  `src/quantizer_lut.rs`
  (`80765f1d036ed9255be0fe6501686b43183864ea852fa66202d8e67fe4848175`).

## Decision

Reject the unchecked lookup, remove the private module, and restore the
crate-wide unsafe prohibition. Samples attributed to the comparison before
the table load were evidently dominated by dependent-load latency or
sampling skid rather than the bounds-check instruction itself. Do not weaken
memory-safety policy for this non-improvement.

## References

- [Research 0014](../research/0014-sampling-and-high-bit-quantization.md)
- [Research 0019](../research/0019-modern-integer-entropy-kernels.md)
- [EXP-0027](EXP-0027-high-bit-quantizer-table.md)
- [EXP-0090](EXP-0090-post-pack-speed-profile.md)
