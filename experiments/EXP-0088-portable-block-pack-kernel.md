# EXP-0088 — Portable word-at-a-time block-pack kernel

Status: **ACCEPTED**

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

The first `u128` prototype was rejected before integration. It improved
packing only from width six upward and made most unpack widths substantially
slower because temporary 16-byte lane construction dominated. A width audit
then found that accepted q90 blocks use only widths six and seven:

| Sample | Winning tiles | Width histogram |
|---|---:|---|
| 10-bit HDR gradient | 72 | 6:8028, 7:72 |
| 10-bit matched motion | 720 | 6:85755, 7:645 |
| 12-bit precision UI | 0 | — |
| 16-bit precision motion | 0 | — |

The revised kernel uses `u64` for widths at most eight and retains scalar
code above that. Across widths one through eight, allocation-reusing
microbenchmarks measured a 2.055x geometric-mean pack speedup and 1.117x
unpack speedup. At the actual widths, pack improved 2.413x/2.519x for
six/seven bits and unpack improved 1.041x/1.087x. No integrated width was
more than 6.6% slower. Exhaustive byte comparison covered legal widths zero
through 17 and lengths zero through 128.

The six-trial matched 10-bit q90 result was:

| Threads | Variant | Encode | Decode | Encoded bytes |
|---:|---|---:|---:|---:|
| 1 | EXP-0086 | 65.395 MP/s | 68.041 MP/s | 18,396,207 |
| 1 | word-at-a-time | 70.302 MP/s | 67.735 MP/s | 18,396,207 |
| 4 | EXP-0086 | 185.510 MP/s | 165.575 MP/s | 18,396,207 |
| 4 | word-at-a-time | 196.066 MP/s | 161.457 MP/s | 18,396,207 |

One-thread encode improved 7.503% and four-thread encode improved 5.691%.
Decode changed -0.450%/-2.487%. All q90 metrics were identical. q100 bytes
were identical and reconstruction remained exact; its focused timings
changed -3.603% encode/-3.948% decode at one thread, within tolerance.

On the complete four-sample q90 supplement, geometric-mean encode improved
1.688% at one thread and 0.835% at four, while decode changed +0.112% and
-1.618%. Complete q100 bytes and metrics were identical; timing changed
-1.665%/-0.688% at one thread and -3.384%/-2.665% at four. The inactive
12/16-bit cases remained byte-identical.

GOP-12 single-frame access geometric means changed +1.606% (10-bit q90),
+1.657% (10-bit q100), -0.782% (16-bit q90), and +0.199% (16-bit q100).
The per-target spread is timing noise around identical dependency and byte
counts; every aggregate is inside 5%.

The new exhaustive byte-oracle test, block round-trip and malformed tests,
strict Clippy, and formatting pass. No unsafe code or ISA flags were added.

Artifacts:

- source commit: `29dcc43`;
- release candidate:
  `artifacts/frontier/fastvid-speed-exp0088-word-block`
  (`adc638be500095ee9dff4e5c8030641178dd5c41517f1a7939d3e77f5a6ec8d7`);
- kernel matrix:
  `artifacts/exp0088-block-pack-kernel.tsv`
  (`048bb25e6b952455adddc774e08960511f333dac5275d23a3c83906ddd89d183`);
- width audit:
  `artifacts/exp0088-block-pack-widths.tsv`
  (`7714431b3c5d0d9b4fada305f3a81813477b794532402847a16f916e9f8513cf`);
- focused matrix:
  `artifacts/exp0088-word-block-focused.tsv`
  (`be69096a720efc0fd1caaa31af7434cd4c4c81349fc8c219369a6544d1d1a60d`);
- complete high-bit matrix:
  `artifacts/exp0088-word-block-highbit.tsv`
  (`4099fc8a8535451796250a36da0a3764de6b284867fceafe636829510c8e7c90`);
- access matrix:
  `artifacts/exp0088-word-block-access.tsv`
  (`c6c0993cafd129ffb9c92bb98515e1d3a7b57de51b87e6c82e35199787e553d4`);
- benchmark source/harness:
  `9b92ec353f620238ad5f1ff9a06ebc0cd0891f3c34eae68cf9721a560ebef62b` /
  `36060262bdc1ce28f101a635f20702eaea7208c9134eeff090e5f378fa846c92`.

## Decision

Accept the portable kernel. It exceeds the isolated and end-to-end gates,
preserves every byte and metric, and improves the packed workload without
architecture coupling. Promote only after a fresh matched OpenAPV slow-tier
run.

Do not add SIMD to this kernel yet: actual width-six unpack is only 4.1% of
the full decode work, while the portable pack already more than doubles its
local throughput. The remaining practical-q90 encode gap is more likely in
prediction/reconstruction and non-packed Rice tiles; profile the new
candidate before choosing the next SIMD target.

## References

- [Research 0034](../research/0034-block-bitpacking-kernels.md)
- [EXP-0086](EXP-0086-sampled-block-pack-format.md)
- [EXP-0087](EXP-0087-block-pack-speed-promotion.md)
