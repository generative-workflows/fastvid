# Modern integer entropy kernels, lookup tables, and exact arithmetic

## Citation trail

This note extends the Rice-coding review in
[research 0005](0005-adaptive-rice-coding.md) and the profiler-directed
hotspots in [research 0012](0012-simd-cache-profiling.md). Sources are limited
to the last ten years, except where a source discusses an older prerequisite.

## Sources

- Lemire, Kurz, and Rupp, [*Stream VByte: Faster Byte-Oriented Integer
  Compression*][stream-paper], 2017/2018, and the [Apache-2.0, stated
  patent-free reference implementation][stream-code].
- Lemire, Kaser, and Kurz, [*Faster Remainder by Direct Computation:
  Applications to Compilers and Software Libraries*][fast-rem], 2019.
- Altamimi and Ben Youssef, [*Lossless and Near-Lossless Compression
  Algorithms for Remotely Sensed Hyperspectral Images*][hsi-rice], 2024.

[stream-paper]: https://arxiv.org/abs/1709.08990
[stream-code]: https://github.com/fast-pack/streamvbyte
[fast-rem]: https://arxiv.org/abs/1902.01961
[hsi-rice]: https://www.mdpi.com/1099-4300/26/4/316

## Control/data separation and shuffle tables

Stream VByte groups four unsigned integers under one control byte, places all
control bytes before the payload bytes, and uses the control byte to index:

- a 256-entry compressed-length table; and
- one of 256 16-byte `pshufb` masks.

Separating control from payload makes future control locations predictable
and removes a dependency present in interleaved group-varint layouts. The
paper reports more than four billion delta-coded integers/s from RAM to L1 on
its Haswell system and up to 2x the comparison decoder. The current reference
implementation has SSE4.1, AArch64 NEON, and scalar paths.

This is not a free Rice optimization. It is an alternate on-wire entropy
format with a fixed quarter-byte control cost per integer and one-to-four
payload bytes. Fastvid residuals often occupy much less than one byte under
Rice coding, while noisy/high-bit tails may favor a byte format. A useful
experiment must first compare modeled and actual bytes by tile and content;
only then should it prototype a tile-local Rice/byte-format choice.

The reusable kernel lesson is smaller: compact control alphabets make
cache-resident length and shuffle lookup tables practical, and separating
control from variable-length data exposes instruction-level parallelism.

## Exact reciprocal arithmetic

The faster-remainder paper computes remainder and divisibility using a
fixed-point reciprocal of a divisor. It derives precision bounds and reports
more than 25% improvement over then-current optimized compiler output for some
remainder cases, and more than 50% for some divisibility tests.

The method is exact within its stated integer domains; it is not an
approximate codec transform. It is relevant only where profiling finds a
runtime divisor or repeated divisor that LLVM has not already strength
reduced. Fastvid's quantizer hot loop already uses an exact lookup table, and
tile dimensions are compile-time/default constants in common cases. Assembly
inspection and a microbenchmark must therefore precede any handwritten
reciprocal implementation.

## Rice-code lookup tables

The 2024 hyperspectral paper cites Rice coding and maps a 16-symbol,
instrument-specific distribution into a 16-byte direct-address table. This
demonstrates a very small codebook lookup, but it is not ordinary Golomb-Rice
parameter selection: symbol order is trained offline for a particular sensor
distribution and the surrounding transform is hyperspectral-specific.

Fastvid can borrow the experimental question, not the table: do a small
fraction of control symbols dominate enough that precomposed `(bits, length)`
entries reduce `finish_entropy` work without changing the bitstream? A table
must be derived exhaustively from the existing normative mapping, remain
exact for all residual widths, and be compared against the accepted stream
byte-for-byte.

## Patent and format guardrails

An open paper is not patent clearance. Stream VByte's maintained reference
repository explicitly describes the approach as patent-free and licenses the
code under Apache-2.0, making it suitable for an isolated comparison. The
other papers provide research ideas only. Any wire-format adoption still
requires a separate format/IP review and a scalar normative specification;
architecture-specific code cannot define the stream.

## Bounded follow-up

1. Record residual magnitude distributions, modeled Rice bytes, and modeled
   Stream-VByte bytes per tile on the standard corpus without changing code.
2. Inspect `finish_entropy` assembly for remaining division/remainder before
   attempting reciprocal arithmetic.
3. Prototype only an exact, bitstream-preserving table for frequently emitted
   Rice fragments; reject it if cache pressure or code layout offsets the
   saved arithmetic.
4. If byte-oriented coding wins a meaningful subset, specify a tile-local
   mode and charge all mode/control bytes before implementing SIMD.

## Relevant experiments

- [EXP-0028: single-pass high-bit Rice cost](../experiments/EXP-0028-single-pass-high-bit-rice-cost.md)
- [EXP-0029: Rice cost early termination](../experiments/EXP-0029-rice-cost-early-termination.md)
- [EXP-0034: perf, Samply, and Cachegrind profile](../experiments/EXP-0034-perf-samply-cache-profile.md)
- [EXP-0036: fused entropy analysis](../experiments/EXP-0036-fused-entropy-analysis.md)
- [EXP-0038: byte-oriented residual modeling](../experiments/EXP-0038-byte-oriented-residual-model.md)
