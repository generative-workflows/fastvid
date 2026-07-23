# EXP-0032 — Rolling high-bit reconstruction state

Status: **ACCEPTED**

## Hypothesis

Replacing the full reconstructed high-bit tile with one rolling row for
spatial prediction, and omitting reconstruction entirely for temporal
prediction, will improve high-bit encode throughput without changing encoded
bytes, quality, compression, or decode performance.

## Modification

Split high-bit tile residual generation into spatial and temporal paths.
Spatial prediction retains only one `u16` row plus scalar left/upper-left
state. Temporal prediction directly folds quantized source/reference
differences and performs no unused reconstruction.

## Test

1. Preserve the EXP-0031 Fastvid binary as the baseline.
2. Require identical encoded bytes across 10/12/16-bit, q90/q100, one/four
   threads, and GOP 1/12.
3. Run the balanced high-bit fast matrix first.
4. Confirm on the full native high-bit supplement only if the fast encode
   geomean improves by at least 3%.
5. Run release tests, strict Clippy, formatting, and the 8-bit regression gate.

## Acceptance criteria

- Every baseline/candidate stream is byte-identical.
- High-bit encode geomean improves by at least 3%.
- No unexplained encode regression exceeds 3%.
- Decode stays within the 5% timing-noise gate.

## Results

The spatial encoder now retains one reconstructed `u16` row (512 bytes for a
256-pixel tile) instead of a complete 256x128 tile (65,536 bytes). The
specialized temporal encoder no longer allocates or writes reconstruction
storage and omits the unused multiply/clamp operation.

The six-trial balanced GOP-1 confirmation covered all four high-bit corpus-v2
supplement samples, q90/q100, and one/four threads:

- encode geomean: **+16.22%** across 16 cells;
- per-cell encode range: **+11.37% to +25.34%**;
- decode geomean: **-0.32%**, with unchanged decoder code;
- encoded sizes and all PSNR/SSIM/error signatures: identical.

The four-trial 10/16-bit video GOP-12 fast matrix improved all eight encode
cells by **8.90% to 28.96%**. Every stream and quality signature was
identical. Decode changes ranged from -6.27% to +2.36%; the isolated
four-thread outlier is timing noise because neither decoder code nor streams
changed.

A balanced 12-image 8-bit q90/one-thread regression produced -0.57% encode and
+0.38% decode geomeans with no size or quality-signature mismatch. This is
inside the 5% gate and the candidate does not modify the 8-bit codec.

Validation passed:

- 26 release tests;
- strict release Clippy;
- formatting and all shell syntax checks;
- the Lean specification build;
- all high-bit corpus-v2 SHA-256 checksums.

Artifacts:

- `artifacts/exp0032-confirm-gop1.tsv`
- `artifacts/exp0032-fast-gop12.tsv`
- `artifacts/exp0032-8bit-regression.tsv`

## Conclusion

Accepted. Reducing predictor reconstruction state and eliminating unused
temporal work materially improves every measured high-bit encode cell while
preserving the exact bitstream, compression, reconstruction, and decoder.
The result supports the working-set hypothesis through wall time, but is not
described as a measured cache-miss reduction because hardware counters were
unavailable.

## References

- [Research 0016](../research/0016-rolling-reconstruction-state.md)
- [EXP-0031](EXP-0031-openapv-matched-baseline.md)
- [Evaluation methodology](../EVALUATION_METHODOLOGY.md)
