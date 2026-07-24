# Modern SIMD rANS implementation and gather limits

## Question

EXP-0068 created four independent rANS states and gained substantially from
scalar instruction-level parallelism. This review asks whether a current,
permissively licensed implementation supports moving directly to AVX2 or
AVX-512 on Fastvid's four-state format.

## Open implementation source

- James Bonfield and contributors,
  [`samtools/htscodecs`](https://github.com/samtools/htscodecs), BSD
  three-clause except for explicitly identified public-domain files.
- The repository's
  [`rANS_static32x16pr_avx2.c`](https://github.com/samtools/htscodecs/blob/master/htscodecs/rANS_static32x16pr_avx2.c)
  and
  [`rANS_static32x16pr_avx512.c`](https://github.com/samtools/htscodecs/blob/master/htscodecs/rANS_static32x16pr_avx512.c)
  implementations.
- Fabian Giesen,
  [`ryg_rans`](https://github.com/rygorous/ryg_rans), public domain, and
  [*Interleaved Entropy Coders*](https://arxiv.org/abs/1402.3392).

Htscodecs is a maintained CRAM codec library. Its older byte-renormalized
format uses four rANS states, while the newer SIMD path uses 32 states with
16-bit renormalization. It dispatches among scalar, SSE4, AVX2, AVX-512, and
NEON implementations.

## What the implementation actually vectorizes

The 32-state AVX-512 decoder keeps two vectors of 16 states. It:

1. masks state slots;
2. gathers packed decode-table entries;
3. emits symbols;
4. computes frequency/shift/bias state updates in vector lanes; and
5. handles renormalization bytes under lane masks.

This is not a four-lane drop-in replacement. Fastvid would need at least an
eight-, sixteen-, or thirty-two-state mode to occupy AVX2/AVX-512 lanes
effectively, increasing every selected tile's final-state charge from 16
bytes to 32, 64, or 128 bytes. It would also need a decode-table layout
designed for vector lookup and a separately verified scalar fallback.

## Gather is not automatically faster

The htscodecs maintainers document a counterintuitive current result:
hardware gather became much slower after Intel's Gather Data Sampling
mitigation, and is also slow on AMD Zen 4. Their default implementation now
simulates gathers with scalar loads and vector construction. They report that
the mitigated hardware-gather path can slow AVX2 decode and AVX-512 encode by
roughly two to three times, while the simulated path can cost 10–30% on older
unpatched CPUs.

The Fastvid benchmark host is AMD EPYC Genoa/Zen 4. Its four-state scalar
batch already performs scalar table lookups followed by independent
arithmetic, closely matching the hybrid lesson from htscodecs. Merely
replacing those lookups with `_mm_i32gather_epi32` would be poorly grounded.

## Fastvid decision boundary

Explicit intrinsics remain possible, but require more evidence than CPU
feature availability:

- a decode-only profile or microbenchmark must isolate table lookup, state
  arithmetic, and renormalization;
- an eight-or-more-state byte model must charge all additional final states;
- real and simulated gather variants need separate Zen 4 measurements;
- runtime dispatch and scalar conformance must preserve identical decoded
  output; and
- unsafe `std::arch` code must be confined to one small module with stated
  slice, alignment, lane, and feature-detection invariants.

EXP-0070's promoted-binary profile does not clear its predeclared
whole-benchmark sampling gate. Combined with htscodecs' 32-state requirement
and gather warning, this argues for exploring a different compression kernel
before introducing unsafe SIMD. It does not prove that a future wider-state
format cannot win.

## Relevant experiments

- [EXP-0068: four-state interleaved rANS](../experiments/EXP-0068-four-state-rans.md)
- [EXP-0070: promoted rANS profile](../experiments/EXP-0070-promoted-rans-profile.md)
