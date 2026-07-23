# EXP-0029 — Rice cost early termination

Status: **ACCEPTED**

## Hypothesis

Stopping high-bit Rice parameter evaluation once the quotient sum reaches zero
will improve encode throughput on low/residual-q90 content while preserving
the exact winning parameter and avoiding the regression from EXP-0028's
nested accumulator.

## Modification

Keep the accepted parameter-outer, compiler-specializable loops. For each
parameter, calculate the quotient sum separately from the fixed
`sample_count * (k+1)` cost. Once the quotient sum is zero, stop: every larger
parameter also has zero quotient cost and a strictly larger fixed cost.

## Test

1. Use the byte-identical restored EXP-0027 binary as baseline.
2. Compare the early-terminating selector with a full 0…16 reference across
   the complete folded domain and representative vectors.
3. Require byte-identical 10/12/16-bit q90/q100 streams.
4. Run four balanced fast trials across the high-bit supplement.
5. Advance to q90/q100, one/four-thread confirmation only for a 3% or greater
   encode geomean improvement with no cell regression beyond 3%.

## Acceptance criteria

- Parameter, cost, and streams are identical to the full scan.
- High-bit encode geomean improves by at least 3%.
- No confirmed encode cell regresses by more than 3%.
- Decode, quality, compression, and the 8-bit path are unchanged.

## Results

The candidate exactly matched the full 0…16 scan for every individual folded
value through 131070 and for complete-domain vectors at five strides. The
release suite passed 26 tests. Direct file comparisons produced
byte-identical streams at 10/12/16 bits and q90/q100; an 8-bit camera stream
was also byte-identical.

Six balanced, serial GOP-1 trials across q90/q100 and one/four threads
measured:

- encode geomean: **+4.32%** across 12 cells;
- q90 per-cell gains: **+4.89% to +9.31%**;
- q100 per-cell range: **-1.13% to +5.22%**;
- decode geomean: **-0.35%**;
- encoded sizes and all reconstruction metrics: unchanged.

The 16-bit GOP-12 video confirmation, also run serially, measured:

| Quality | Threads | Baseline encode | Candidate encode | Change | Decode change |
|---:|---:|---:|---:|---:|---:|
| 90 | 1 | 68.293 MP/s | 80.887 MP/s | +18.44% | -1.33% |
| 90 | 4 | 127.920 MP/s | 158.971 MP/s | +24.27% | +0.41% |
| 100 | 1 | 52.710 MP/s | 53.195 MP/s | +0.92% | +2.10% |
| 100 | 4 | 118.864 MP/s | 121.847 MP/s | +2.51% | +4.24% |

GOP-12 encode geomean improved **11.09%** and decode geomean moved **+1.33%**.
The positive 4-thread q100 decode outlier is unmodified-code timing variation,
not a decoder claim.

Artifacts:

- `artifacts/exp0029-fast.tsv`
- `artifacts/exp0029-confirm-gop1.tsv`
- `artifacts/exp0029-confirm-gop12.tsv`

## Conclusion

Accepted. A mathematically terminal quotient-zero condition retains LLVM's
fast parameter-specialized loops while avoiding provably useless larger
parameters. It improves the high-bit encode matrix without changing any
stream, quality, compression, decoder, or 8-bit behavior.

## References

- [Research 0014](../research/0014-sampling-and-high-bit-quantization.md)
- [EXP-0027](EXP-0027-high-bit-quantizer-table.md)
- [EXP-0028](EXP-0028-single-pass-high-bit-rice-cost.md)
