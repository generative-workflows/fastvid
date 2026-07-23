# EXP-0033 — Incremental high-bit decoder coordinates

Status: **REJECTED**

## Hypothesis

Passing raster coordinates from the decode loops instead of deriving them with
runtime remainder/division inside every reconstruction call will improve
native high-bit decode throughput without changing accepted streams or output.

## Modification

Iterate Rice-coded tiles by explicit row/column coordinates. Maintain
incremental coordinates in zero-run decoding. Change the reconstruction helper
to consume validated `index`, `x`, and `y` values directly.

## Test

1. Preserve the accepted EXP-0032 binary as baseline.
2. Run release malformed/conformance tests and strict Clippy.
3. Require identical size and quality signatures on the balanced high-bit
   video fast matrix.
4. Advance to six-trial full-supplement confirmation if decode geomean
   improves by at least 3%.
5. Reject and revert if the fast gate regresses or remains inside noise.

## Acceptance criteria

- All baseline/candidate reconstruction signatures are identical.
- High-bit decode geomean improves by at least 3%.
- No unexplained decode cell regresses by more than 3%.
- Encode remains within the 5% noise gate because encoder code is unchanged.

## Results

The four-trial video fast matrix initially passed:

- decode geomean: **+3.89%** across eight cells;
- decode range: +1.44% to +6.65%;
- encode geomean: -2.35%;
- encoded sizes and reconstruction signatures: identical.

The six-trial full-supplement confirmation measured **+3.33%** decode and
**-2.73%** encode geomeans across 16 cells. Decode changes ranged from -2.18%
to +9.14%. Although encoder source was unchanged, the 12-bit UI q100/four-
thread encode cell regressed 8.11% and the 16-bit motion q100/four-thread cell
regressed 5.19%.

Ten-trial focused reruns did not clear those regressions:

| Cell | Encode change | Decode change |
|---|---:|---:|
| 12-bit UI q100, 4 threads | -10.35% | +5.21% |
| 10-bit gradient q100, 4 threads | -7.67% | +2.75% |
| 16-bit motion q100, 4 threads | -3.19% | +3.24% |
| 10-bit motion q100, 4 threads | -4.57% | +1.41% |

ThinLTO and whole-binary code layout can affect unchanged functions; the
delivered binary regression remains relevant even when the source-level cause
is indirect. The candidate therefore fails the no-unexplained-regression gate.

Artifacts:

- `artifacts/exp0033-fast-gop1.tsv`
- `artifacts/exp0033-confirm-gop1.tsv`
- `artifacts/exp0033-focused-stills-q100-t4.tsv`
- `artifacts/exp0033-focused-video-q100-t4.tsv`

## Conclusion

Rejected and reverted. Incremental coordinates improved high-bit decode by
about 3.3%, but the complete candidate binary reproducibly lost substantially
more encode throughput on 1080p stills. A future retry should isolate the
decoder into a codegen boundary or inspect final binary layout before
reconsidering the transformation.

## References

- [Research 0017](../research/0017-decoder-coordinate-strength-reduction.md)
- [EXP-0032](EXP-0032-rolling-high-bit-reconstruction.md)
