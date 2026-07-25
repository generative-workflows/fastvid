# EXP-0088 — Portable word-at-a-time block-pack kernel

Status: **PENDING**

## Classification

**Kernel exploitation with fast feedback** — optimize the accepted EXP-0086
fixed-block syntax before introducing architecture-specific SIMD.

## Hypothesis

Eight fixed-width symbols occupy exactly `width` bytes. For widths 1 through
16, packing those symbols into one `u128` removes per-symbol byte flushing and
cross-group bit state while preserving the normative LSB-first byte stream.
A portable eight-symbol pack/unpack kernel should improve isolated scalar
throughput by at least 20% and recover the EXP-0086 focused encode cost
without changing bytes or quality.

## Modification

1. Add a deterministic allocation-reusing microbenchmark for 128-symbol
   blocks across representative widths.
2. Compare the current scalar bit-buffer shape with an eight-symbol `u128`
   kernel for packing and unpacking.
3. Exhaustively verify byte identity and decoded values for widths 0 through
   17, including partial final groups.
4. If the microbenchmark passes, integrate the portable kernel for widths at
   most 16 and retain the current scalar path for width 17 and remainders.
5. Do not add unsafe code, target-wide ISA flags, or architecture dispatch in
   this experiment.

The benchmark must reuse output storage so it measures the kernel rather than
allocator behavior. `std::hint::black_box` prevents dead-code elimination.

## Fast gate

- exact packed bytes and decoded symbols for every legal width and block
  length exercised by tests;
- at least 20% geometric-mean isolated pack or unpack improvement, with no
  representative width slower by more than 10%;
- focused matched-q90 bytes and metrics identical to EXP-0086;
- focused one-thread encode improves at least 2% versus EXP-0086 or reaches
  the original EXP-0078 throughput while retaining the rate saving;
- focused decode does not regress more than 5%;
- q100 remains byte-identical and exact; and
- strict Clippy, formatting, malformed-stream, and round-trip tests pass.

Passing the microbenchmark alone does not advance SIMD or the frontier.

## Result

Pending.

## Decision

Pending.

## References

- [Research 0034](../research/0034-block-bitpacking-kernels.md)
- [EXP-0086](EXP-0086-sampled-block-pack-format.md)
- [EXP-0087](EXP-0087-block-pack-speed-promotion.md)
