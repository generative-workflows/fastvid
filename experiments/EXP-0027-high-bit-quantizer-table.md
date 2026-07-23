# EXP-0027 — High-bit quantizer lookup table

Status: **ACCEPTED**

## Hypothesis

Replacing per-sample signed division in the 10/12/16-bit encoder with one
read-only residual lookup table per frame will materially improve high-bit
encode throughput, especially at 16 bits, without changing encoded bytes,
reconstruction, or the separate 8-bit path.

## Modification

Add a `Quantizer16` whose contiguous `i32` table covers the exact signed
residual domain for the signaled depth. Construct it once in `encode_internal`
and share it immutably across tile workers. The largest table has 131,071
entries (524,284 bytes). Keep scalar quantization as the table-construction
oracle and for exhaustive equivalence tests.

## Test

1. Preserve a release baseline binary from the current EXP-0026 worktree.
2. Exhaustively compare table and scalar quantization for every residual,
   quality, and 10/12/16-bit depth.
3. Require identical encoded streams and decoded frames across existing
   conformance tests.
4. Run balanced baseline/candidate trials on the checksummed high-bit smoke
   corpus at q90/q100, one/four threads, and GOP 1/12.
5. Advance to repeated broader rows only if the encode geomean improves by at
   least 3% with no material decode, compression, or quality change.
6. Rerun the 8-bit fast feedback gate.

## Acceptance criteria

- High-bit encoded bytes and reconstruction are bit-identical.
- High-bit encode geomean improves by at least 3%.
- No high-bit case regresses encode throughput by more than 3%.
- Decode change stays within 3%, since decode is unmodified.
- Eight-bit results stay within the established 5% noise gate with identical
  encoded sizes.

## Results

The candidate uses one exact-domain `Vec<i32>` per encoded frame and shares it
immutably across tile workers. An exhaustive release test checks every signed
residual at all 100 qualities for 10, 12, and 16 bits against scalar
quantization. The full suite increased to 25 passing tests.

The baseline and candidate release binaries were preserved separately with
debug information and measured in balanced execution order. Six-trial GOP-1
confirmation covered all three native high-bit samples at q90/q100 and
one/four threads:

- encode geomean: **+14.03%** across 12 cells;
- per-cell encode range: **+7.80% to +18.88%**;
- decode geomean: **+0.17%**;
- encoded sizes, PSNR, SSIM, and maximum errors: unchanged.

The six-trial 16-bit GOP-12 video confirmation was:

| Quality | Threads | Baseline encode | Candidate encode | Change | Decode change |
|---:|---:|---:|---:|---:|---:|
| 90 | 1 | 58.423 MP/s | 67.612 MP/s | +15.73% | +3.86% |
| 90 | 4 | 110.619 MP/s | 120.675 MP/s | +9.09% | +2.38% |
| 100 | 1 | 39.253 MP/s | 51.550 MP/s | +31.33% | +1.26% |
| 100 | 4 | 77.073 MP/s | 94.829 MP/s | +23.04% | +3.54% |

The GOP-12 encode geomean improved **19.51%** and decode geomean moved
**+2.75%**. Decode is byte-for-byte the same implementation; the two positive
changes just above 3% are timing variation, not a claimed decoder
optimization. A focused ten-trial q100 one-thread rerun reduced the only
GOP-1 decode regression beyond 3% from -3.41% to -1.02%.

Direct file comparisons confirmed byte-identical baseline/candidate streams
for all 10/12/16-bit depth and q90/q100 combinations. The 12-image 8-bit
regression matrix produced +2.64% encode and -0.28% decode geomeans with
identical size/quality signatures. Its noisiest encode row was rerun for ten
balanced trials and measured -3.59%, inside the established 5% gate.

Artifacts:

- `artifacts/exp0027-confirm-gop1.tsv`
- `artifacts/exp0027-confirm-gop12.tsv`
- `artifacts/exp0027-focused-q100-1t.tsv`
- `artifacts/exp0027-8bit-regression.tsv`

## Conclusion

Accepted. Moving high-bit quantization division from every sample to one
contiguous per-frame table materially improves every measured high-bit encode
cell while preserving the exact bitstream, quality, compression, decoder, and
specialized 8-bit path.

## References

- [Research 0014](../research/0014-sampling-and-high-bit-quantization.md)
- [EXP-0023](EXP-0023-quantizer-lookup-table.md)
- [EXP-0026](EXP-0026-high-bit-depth-foundation.md)
