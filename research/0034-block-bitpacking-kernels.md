# Block bit-packing kernels and runtime dispatch

## Citation trail

This is a forward-citation and implementation pass from
[research 0019](0019-modern-integer-entropy-kernels.md) and the negative
writer experiments EXP-0080/0083/0084. The older SIMD-BP128 paper is retained
as the base algorithm; the implementation and forward evidence are current
or from the last ten years.

## Open sources

- Lemire and Boytsov, [*Decoding billions of integers per second through
  vectorization*](https://arxiv.org/abs/1209.2137), the SIMD-BP128 base paper.
- The maintained [FastPFOR reference
  implementation](https://github.com/fast-pack/FastPFor), Apache-2.0. Its
  authors state that `SIMDBinaryPacking` operates on 128-integer blocks and
  document portable, native, and experimental runtime-dispatch build modes.
- Mendis, Jain, Jain, and Amarasinghe,
  [*Revec: Program Rejuvenation through
  Revectorization*](https://www.parasjain.com/projects/19revec/paper.pdf),
  CC 2019.
- Liao et al., [*SFVInt: Simple, Fast and Generic Variable-Length Integer
  Decoding using Bit Manipulation
  Instructions*](https://arxiv.org/abs/2403.06898), 2024.

Fastvid does not copy these implementations. FastPFOR is an openly usable
design/code reference. SFVInt is decoder evidence only; BMI2 varint
instruction techniques need a separate patent review before format use.

## Findings

SIMD-BP128 chooses one bit width for a block of 128 integers and stores the
values in a vertical layout suitable for SIMD packing/unpacking. The paper's
realistic-data tables report especially high encode/decode rates for binary
packing, but also a measurable rate gap from more adaptive patched formats.
The important structural contrast with Fastvid Rice is that one small block
control replaces a data-dependent unary length for every symbol.

FastPFOR's maintained documentation rejects the premise that SIMD formats
require thousand-value blocks: its fastest binary packer uses 128. It also
separates portable and native deployment modes and warns that native binaries
are not distributable. This independently supports EXP-0082's decision that
target-wide `native` builds are diagnostics, not a codec dispatch strategy.

Revec is useful forward evidence about kernel shape. FastPFOR contains a
specialized unpacking routine for each bit width. Revec reports a 1.160x
geometric-mean improvement when retargeting those kernels to AVX2 and 1.430x
to AVX-512 on its Skylake server. It also reports that stock Clang sometimes
slows down when AVX-512 is merely enabled, matching Fastvid's finding that ISA
flags alone do not vectorize dependency-heavy code.

SFVInt obtains up to 2x LEB128 decode gains by grouping control decisions and
using BMI2. Its AMD section is the guardrail: PEXT/PDEP behavior varies by
microarchitecture, and its advantage is smaller for predominantly one-byte
values. This argues for runtime-dispatched microbenchmarks, not unconditional
BMI2 in a portable decoder.

## Fastvid implications

A tile-wide maximum bit width is likely too expensive for noisy outliers.
The correct first model is smaller and fully charged:

1. retain fixed clamp-gradient reconstruction and fold residuals exactly;
2. partition each tile into causal order blocks of 128 symbols;
3. signal a five-bit width for each block;
4. charge `ceil(count * width / 8)` payload bytes plus every control byte;
5. compare against exact zero-run/Rice bytes per tile and sample; and
6. separately report blocks where width 0, fixed packing, or Rice wins.

This is initially a size model, not a speed claim. A candidate is interesting
only if its matched q90 increase fits inside the current rate margin to
OpenAPV and it wins on a meaningful fraction of camera/motion blocks. If it
passes, a scalar normative packer should precede architecture-specific
AVX2/AVX-512 kernels and runtime dispatch.

The model must not assume that the 128-symbol control is free, silently patch
outliers, or compare database deltas directly with image residuals. Any
patched exception stream is a separate experiment with its own controls and
patent review.

## Relevant experiments

- [EXP-0038: byte-oriented residual model](../experiments/EXP-0038-byte-oriented-residual-model.md)
- [EXP-0079: unified speed profile](../experiments/EXP-0079-unified-speed-profile.md)
- [EXP-0080: inlined Rice writer](../experiments/EXP-0080-inlined-rice-writer.md)
- [EXP-0083: four-symbol Rice batching](../experiments/EXP-0083-four-symbol-rice-batching.md)
- [EXP-0084: specialized Rice batching](../experiments/EXP-0084-specialized-rice-batching.md)
- [EXP-0085: charged block bit-packing model](../experiments/EXP-0085-block-bitpacking-model.md)
- [EXP-0086: sampled block-pack format](../experiments/EXP-0086-sampled-block-pack-format.md)
- [EXP-0087: block-pack speed promotion](../experiments/EXP-0087-block-pack-speed-promotion.md)
