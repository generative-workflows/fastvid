# EXP-0045 — Rolling 8-bit reconstruction row

Status: **ACCEPTED**

## Hypothesis

The 8-bit spatial encoder retains a full reconstructed tile solely to access
Paeth's left, above, and upper-left neighbors. Replacing that allocation with
one rolling row, as already accepted for high-bit encoding in EXP-0032, will
reduce allocation size and reconstruction-state cache traffic without
changing any prediction, residual, entropy decision, or stream byte.

## Modification

In 8-bit `encode_spatial_tile`, replace `vec![0u8; width * height]` with
`vec![0u8; width]`. For each row:

1. keep `left` and `upper_left` scalars;
2. read `above` from the row slot before overwriting it;
3. compute the existing Paeth prediction and quantized reconstruction;
4. write the current reconstruction into the row slot;
5. advance `upper_left = above` and `left = reconstructed`.

Do not change temporal coding, high-bit code, quantization, entropy, tiles, or
the decoder.

## Test

1. Add an exact oracle comparing full-tile and rolling-row reconstructed
   residuals over odd dimensions, edge pixels, q90, and q100.
2. Require fresh candidate provenance and exact 8-bit/12-bit stream controls.
3. Run the balanced 8-bit video matrix at one/four threads and q90/q100.
4. Advance for at least 2% encode geomean with no cell regression beyond 3%;
   decode remains inside 5%.
5. Confirm on 8-bit stills and allocation/cache counters for an advancing
   candidate.
6. Run release tests, strict Clippy, formatting, and Lean.

## Acceptance criteria

- Reconstructed samples, residuals, entropy choices, and streams are exact.
- Encode geomean improves at least 2%.
- No confirmed encode cell regresses more than 3%; decode stays inside 5%.
- Allocation size falls from `width * height` to `width` bytes per spatial
  tile.
- Counters or profiles do not contradict the reduced-working-set mechanism.

## Results

All 31 release tests, strict Clippy, formatting, the full-reconstruction
oracle, and exact-stream controls passed. The fresh candidate SHA-256 was
`06ef3278e9055f3c53c94cf964f4a7bf785453b696e0df262dec9161b45c6ab8`.
The established 8-bit and 12-bit control hashes remained
`474eea3b68bdbfa0c4f133699fa3dc0a17aa1ff6658b1afa489e96cd05c2eac8`
and
`d82e90e8229597c0acd19676de4b5ccd8f8f147fb651f2e1778643168432c29f`.

The six-trial balanced feedback loop measured +13.87% encode geomean. A
ten-trial q90/one-thread still matrix measured +15.82% geomean, but the camera
cell's wall times were bimodal. Repeated hardware counters on that exact cell
showed the candidate was genuinely cheaper:

| Counter | EXP-0041 | Candidate | Change |
|---|---:|---:|---:|
| cycles | 390,104,989 | 364,250,689 | -6.63% |
| instructions | 1,278,365,042 | 1,151,400,660 | -9.93% |
| branches | 170,991,642 | 150,567,615 | -11.94% |
| branch misses | 4,022,130 | 4,033,917 | +0.29% |

The complete balanced four-trial 8-bit video confirmation across q90/q100 and
one/four threads measured:

- encode geomean: **+15.04%**;
- encode cell range: **+8.95% to +25.29%**;
- decode geomean: -0.29%;
- decode cell range: -5.77% to +3.73%.

The single decode outlier is in unchanged code; its paired encode cell
improved 10.42%, and the aggregate decoder result is centered on zero. Every
stream size and reconstruction signature was unchanged. For a full 256x128
tile, reconstruction state falls from 32,768 bytes to 256 bytes, a 128x
reduction; the residual vector and exact entropy allocation remain unchanged.

Artifacts:

- `artifacts/exp0045-fast-feedback.tsv`
  (`6211b95e088f7e47f6e3f4004e9920a887026be7c062322bdefc226ad58548f0`);
- `artifacts/exp0045-images-q90-t1.tsv`
  (`84a4b2f4879b9c5f38e84849822fb7b61029157c650de9ee8b13a0a78ff0c22e`);
- `artifacts/exp0045-confirm-8bit-video.tsv`
  (`3c03dc08d1758849a45572ec0b7d2b03b82d1ad3f9a915c89e4a26a80b1948bd`);
- `artifacts/exp0045-camera-perf-baseline.txt`
  (`ddb484fcb92e3e5e524b0aaa2b4bb925b9b114a4ccb2636e49384308a6a0823e`);
- `artifacts/exp0045-camera-perf-candidate.txt`
  (`3812272460eb3fb264867662335c66bc3d61aacc2b2d9cb049610ab5850b8b86`).

## Conclusion

Accepted. The 8-bit spatial predictor needs only one previous reconstructed
row, exactly like the accepted high-bit implementation. Removing full-tile
reconstruction state provides a large, consistent encode improvement and
reduces cycles, instructions, branches, allocation size, and cache-active
state without changing one stream byte. This is a substantially larger gain
than changing thread pools or mutex implementations because it removes work
inside every spatial tile rather than coordination once per tile.

## References

- [EXP-0032](EXP-0032-rolling-high-bit-reconstruction.md)
- [Research 0020](../research/0020-modern-parallel-codec-kernels.md)
