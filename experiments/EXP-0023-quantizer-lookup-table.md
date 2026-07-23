# EXP-0023 — Frame-local quantizer lookup table

Status: **REJECTED**

## Hypothesis

Replacing per-sample signed integer division with a 511-entry lookup table will
improve spatial and temporal encoding while preserving the exact quantizer
mapping and bitstream.

## Modification

Build one table per encoded frame for every residual in `[-255, 255]`. Share it
immutably across tile workers and index it from spatial and temporal residual
loops. The table occupies 1,022 bytes using `i16`, fitting in the host L1 data
cache; construction performs only 511 divisions per frame.

## Test

1. Prove table equality against scalar `quantize` for all residuals and every
   legal quality-derived step.
2. Run two fast-tier five-trial candidate matrices.
3. Require identical encoded sizes and quality.
4. Confirm an accepted candidate with balanced A/B corpus testing.

## Acceptance criteria

- Both fast-tier encode geomeans improve by at least 5%.
- No fast-tier encode case regresses.
- Decode remains within 2%, and output is bit-identical.

## Results

Against the post-EXP-0021 fast baselines, candidate encode geomeans were
77.617 and 77.480 MP/s versus 65.779 and 66.434 MP/s, improvements of 17.99%
and 16.63%. Every individual encode case improved and encoded bytes matched.

Decode geomeans were 103.376 and 102.504 MP/s versus 105.201 and
105.280 MP/s. The second comparison moved -2.64%, narrowly failing the 2%
unchanged-path bound.

## Conclusion

Rejected under its fast-tier negative-control gate. The encode result is large
enough for balanced corpus confirmation in
[EXP-0024](EXP-0024-quantizer-table-confirmation.md).


## References

- [Research 0012](../research/0012-simd-cache-profiling.md)
- [EXP-0022](EXP-0022-llvm-vectorization-audit.md)
