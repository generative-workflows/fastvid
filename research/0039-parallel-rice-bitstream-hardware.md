# Parallel Rice bitstream hardware

## Question

What recent hardware evidence applies to Fastvid's remaining Rice emission and
variable-output bottlenecks, and which parts are compatible with an MIT
implementation?

## Sources

- Žan Regoršek, Aleš Gorkič, and Andrej Trost,
  [*Parallel Lossless Compression of Raw Bayer Images on FPGA-Based
  High-Speed Camera*](https://www.mdpi.com/1424-8220/24/20/6632),
  *Sensors* 24(20), 6632, 2024, DOI 10.3390/s24206632. The article is CC-BY
  4.0.
- The authors'
  [OMLS reference repository](https://github.com/joej970/OMLS), GPL-3.0.

The paper is used as openly readable architecture and measurement evidence.
The GPL implementation is not compatible with Fastvid's intended MIT license:
no source, constants, tables, or implementation structure are copied.

## Findings

### Parallel entropy needs a parallel bitstream packer

The design accepts 16 eight-bit pixels per clock at 320 MHz. Each group can
produce fewer or more than the fixed 128-bit output width, so the authors
combine codewords with a tree-like pipeline and buffer/align the result to
128-bit transfers. They explicitly reject sequential index calculation and
concatenation as a critical-path bottleneck.

This is direct hardware evidence for separating Fastvid version 5 into:

1. independent lane-local code generation with exact bit counts;
2. a bounded prefix sum or tree scan of lane/shard sizes;
3. fixed-width packing into disjoint output words; and
4. a small boundary buffer rather than a shared append stream.

Version 5 already supplies byte-delimited Rice lanes and shard lengths, so it
does not require bit-exact concatenation across all 4,096 symbols. A CPU
implementation should specialize several values at once; a future CUDA
implementation should have each warp produce bounded lane fragments and use
scan-assigned byte ranges.

### Bound pathological unary codes

The FPGA format limits the unary quotient to eight bits and falls back to an
identifier plus an 11-bit literal, bounding a codeword at 19 bits. That bound
enables a fixed pipeline and prevents a rare residual from stalling bitstream
generation.

Fastvid currently caps residual magnitude but a Rice-0 code can still be very
long. Fixed-block competition bounds the selected *shard* body, but the Rice
writer still constructs a losing pathological lane before selection. A
charged escape/literal Rice syntax is therefore a plausible exploration
branch for both rate robustness and GPU register/output bounds. It is a
format change and must be modeled against fixed block first; the paper's
specific eight-bit threshold is fitted to 8-bit Bayer data and is not a
Fastvid constant.

### Throughput and scope

The FPGA reports 40.10 Gbit/s effective input throughput, 5,120 Mpixel/s,
320 MHz, and 16 pixels per cycle. High-resolution camera images reach up to
2.26x lossless compression; Kodak averages about 1.34x. These numbers are not
comparable to Fastvid or OpenAPV because the source is 8-bit Bayer CFA, the
quality is lossless, the hardware is an AMD-Xilinx FPGA, and rate is
deliberately traded for a shallow pipeline.

The useful target is architectural: every stage operates continuously without
a state machine, output packing is explicitly parallel, and worst-case code
length is bounded. Fastvid should record cycles or pixels per produced group
and maximum codeword/lane output in addition to MP/s.

## Design consequences

1. Preserve version 5's independent byte-aligned lanes; do not merge them
   into a single tile-wide Rice state to save a few length bytes.
2. Optimize Rice emission in fixed groups and pre-size output from exact bit
   counts.
3. Model a bounded unary escape before specifying it. Compare complete bytes
   and warp divergence against fixed block, not only average Rice bits.
4. Prototype scan/disjoint-write assembly on CPU before CUDA so the canonical
   output contract no longer depends on a serial append loop.
5. Do not use the GPL OMLS source. Re-derive any implementation solely from
   Fastvid's format and general prefix-scan/bit-packing identities.

## Relevant experiments

- [EXP-0110](../experiments/EXP-0110-full-tile-bounded-shards.md)
- [EXP-0112](../experiments/EXP-0112-version5-encode-profile.md)
- [EXP-0113](../experiments/EXP-0113-parallel-rice-early-termination.md)
- [EXP-0114](../experiments/EXP-0114-parallel-rice-grouped-emission.md)
