# EXP-0028 — Single-pass high-bit Rice cost

Status: **REJECTED**

## Hypothesis

Computing all 17 high-bit Rice quotient sums in one folded-residual traversal
will improve high-bit encode throughput by reducing repeated tile-vector
loads, without changing the selected parameter, payload, quality, or decoder.

## Modification

Replace the parameter-outer `best_rice_parameter` scan with one sample-outer
pass over the folded `Vec<u32>`. Accumulate quotient sums for parameters 0…16
in a fixed stack array, then add each parameter's constant unary/remainder
cost. Do not alter residual generation or entropy emission.

## Test

1. Use the accepted EXP-0027 binary as the preserved baseline.
2. Exhaustively compare the old and new cost/parameter result for individual
   folded values throughout the complete 16-bit residual domain and for
   representative mixed vectors.
3. Require byte-identical streams at 10/12/16 bits and q90/q100.
4. Run the balanced high-bit fast matrix, then the full high-bit confirmation
   only if encode geomean improves by at least 3%.
5. Rerun an 8-bit regression sample even though its code path is unchanged.

## Acceptance criteria

- Selected Rice parameters and encoded streams are identical.
- High-bit encode geomean improves by at least 3%.
- No high-bit encode cell regresses more than 3%.
- Decode remains within the 3% measurement-noise band.
- Eight-bit behavior and size remain unchanged.

## Results

The reference-equivalence test passed across every individual folded value
from 0 through 131070 and a mixed-domain vector. All codec tests passed and
encoded byte counts were unchanged.

Four balanced q90, one-thread trials on the high-bit smoke corpus measured:

| Depth | Baseline encode | Candidate encode | Change |
|---:|---:|---:|---:|
| 10 | 26.401 MP/s | 26.009 MP/s | -1.48% |
| 12 | 32.437 MP/s | 31.821 MP/s | -1.90% |
| 16 | 40.694 MP/s | 39.704 MP/s | -2.43% |

The encode geomean regressed **1.94%**. Decode variation was unrelated to the
change, and encoded sizes remained 1,870,568, 1,260,044, and 3,637,139 bytes.
The fast gate therefore rejected the candidate without spending the larger
confirmation budget. Full rows are preserved in
`artifacts/exp0028-fast.tsv`.

## Conclusion

Rejected and reverted. Reducing folded-vector traversals did not offset the
cost of the sample-outer nested accumulator. The parameter-outer loops give
LLVM simpler constant-shift kernels and remain faster on this host. Future
entropy work should preserve those specialized loops or demonstrate a
different vectorized cost formulation.

## References

- [Research 0014](../research/0014-sampling-and-high-bit-quantization.md)
- [EXP-0027](EXP-0027-high-bit-quantizer-table.md)
