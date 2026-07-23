# EXP-0035 — Narrow folded-residual storage

Status: **REJECTED**

## Hypothesis

Storing tile-local folded residuals as `u16` whenever the exact quantized
range fits will reduce `finish_entropy` cache traffic and improve high-bit
encode throughput without changing entropy decisions, payload bytes, quality,
or decode performance.

## Modification

Make residual generation and entropy finishing generic over an internal
folded-value type. Select `u16` once per tile when the maximum possible
quantized zigzag value is at most 65,535; otherwise retain `u32`. This keeps
16-bit q100's full residual range while narrowing all current 10/12-bit rows
and lossy 16-bit rows.

No stream syntax or public API changes.

## Test

1. Preserve the accepted EXP-0032 binary as baseline.
2. Exhaustively verify that the narrow conversion bound covers every
   quantized residual at all 10/12/16-bit qualities.
3. Require byte-identical streams and quality signatures on the balanced
   video fast matrix at q90/q100, one/four threads, and GOP 1/12.
4. Advance to six-trial full-supplement confirmation only if encode geomean
   improves by at least 3%.
5. For an accepted wall-time result, repeat the EXP-0034 one-thread q90 PMU
   run and compare L1D loads/misses, cycles, and instructions.
6. Run release tests, strict Clippy, formatting, Lean, and 8-bit regression.

## Acceptance criteria

- Every stream and reconstruction signature is identical.
- High-bit encode geomean improves by at least 3%.
- No unexplained encode cell regresses more than 3%.
- Decode and 8-bit changes remain inside the 5% noise gate.
- PMU counts do not contradict the proposed working-set mechanism.

## Results

All 26 release tests and the exhaustive range/conversion check passed. Every
fast-matrix stream and reconstruction signature was byte-identical.

The four-trial balanced video matrix nevertheless failed:

| Cell | Encode change | Decode change |
|---|---:|---:|
| 10-bit q90, 1 thread | -11.46% | -2.17% |
| 10-bit q90, 4 threads | -10.25% | -0.34% |
| 10-bit q100, 1 thread | -12.79% | +1.78% |
| 10-bit q100, 4 threads | -10.22% | -0.60% |
| 16-bit q90, 1 thread | -3.88% | -0.14% |
| 16-bit q90, 4 threads | -4.49% | -0.41% |
| 16-bit q100, 1 thread | -12.02% | +0.14% |
| 16-bit q100, 4 threads | -11.63% | -1.24% |

Encode geomean regressed **9.65%**. The 16-bit q100 rows deliberately retained
`u32`, yet regressed by roughly 12%; this control shows that generic
monomorphization/conversion structure and whole-binary layout dominated any
narrow-vector locality benefit. Decode geomean moved -0.38%.

Artifact: `artifacts/exp0035-fast-gop1.tsv`.

## Conclusion

Rejected and reverted at the fast gate. Smaller element width is not useful
through this generic dual-representation implementation. A future cache
experiment should avoid duplicating the complete entropy hot path—prefer a
single-representation pass reduction or a separately codegen-isolated helper.
No PMU confirmation was run because wall time already decisively failed.

## References

- [EXP-0034](EXP-0034-perf-samply-cache-profile.md)
- [Research 0014](../research/0014-sampling-and-high-bit-quantization.md)
