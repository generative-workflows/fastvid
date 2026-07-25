# Parallel-hardware-friendly codec structure

## Question

Which format properties let an intermediate image/video codec map efficiently
to GPUs and other wide parallel hardware, rather than merely wrapping a
serial codec in a parallel API?

This is a deeper follow-up to
[research 0020](0020-modern-parallel-codec-kernels.md). That review established
that dependency removal precedes SIMD. This pass compares the actual
independence boundaries, entropy layouts, and output-assembly mechanisms used
by recent codecs and GPU compression formats.

## Primary sources and use constraints

- Taubman, Naman, and Mathew,
  [*High Throughput Block Coding in the HTJ2K Compression
  Standard*](https://kakadusoftware.com/wp-content/uploads/icip2019.pdf),
  ICIP 2019; and the
  [OpenJPH reference implementation](https://github.com/aous72/OpenJPH),
  BSD-2-Clause.
- Rhatushnyak et al.,
  [*Committee Draft of JPEG XL Image Coding
  System*](https://arxiv.org/abs/1908.03565), 2019; the
  [JPEG XL white paper](https://ds.jpeg.org/whitepapers/jpeg-xl-whitepaper.pdf);
  and [libjxl](https://github.com/libjxl/libjxl), BSD-3-Clause with its
  additional IP-rights grant.
- JPEG committee,
  [*JPEG XS white paper*](https://ds.jpeg.org/whitepapers/jpeg-xs-whitepaper.pdf);
  and Richter et al.,
  [*JPEG XS — A New Standard for Visually Lossless Low-Latency
  Lightweight Image Coding*](https://dial.uclouvain.be/pr/boreal/object/boreal%3A248275/datastream/PDF_01/view),
  2021.
- NVIDIA,
  [GDeflate stream definition](https://docs.nvidia.com/cuda/nvcomp/gdeflate.html);
  and Microsoft,
  [DirectStorage GDeflate reference
  implementation](https://github.com/microsoft/DirectStorage), MIT.
- NVIDIA,
  [CUDA C++ Best Practices Guide](https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html);
  Harris, Sengupta, and Owens,
  [*Parallel Prefix Sum (Scan) with
  CUDA*](https://developer.nvidia.com/gpugems/gpugems3/part-vi-gpu-computing/chapter-39-parallel-prefix-sum-scan-cuda).
- Tian et al.,
  [*Revisiting Huffman Coding: Toward Extreme Performance on Modern GPU
  Architectures*](https://arxiv.org/abs/2010.10039), 2020.

JPEG XS is reviewed as comparative architecture evidence only. The reviewed
papers and committee documents do not themselves establish MIT-compatible
patent rights, so Fastvid must not copy its normative coding tools without a
separate clearance. Actionable implementation details below are restricted to
general parallel algorithms or the compatibly licensed OpenJPH, libjxl, and
DirectStorage sources.

## What the successful designs make explicit

### Independent spatial units

HTJ2K partitions wavelet subbands into independently coded code-blocks. Its
high-throughput block coder replaces the original JPEG 2000 block coder while
retaining region/resolution access. The 2019 paper emphasizes vectorization
and independent code-blocks rather than one adaptive state spanning the
picture.

JPEG XL divides large images into at-most 256x256 groups, stores an index of
their bitstream positions, and processes AC groups in parallel. Independence
costs some cross-group modeling opportunity, but it simultaneously bounds
working memory, supports region access, and supplies enough tasks for
multicore decoding.

The lesson is stronger than “use tiles”: report both the number of schedulable
units and the longest serial dependency inside one unit. A 32K-sample tile
with one causal predictor and one variable-length entropy state is still a
large serial job even if hundreds of tiles can run independently.

### Entropy layout is part of the parallel contract

Ordinary Huffman/DEFLATE decoding cannot find the next variable-length symbol
without decoding the current one. GDeflate changes the stream layout itself:
it serializes 32 logical substreams in a defined refill order, so a warp keeps
32 independent 64-bit states while preserving essentially the same DEFLATE
codes and compression ratio. The GPU Huffman paper similarly uses parallel
codebook construction and reduction-based codeword packing; a serial
bit-writer would erase much of the available parallelism.

HTJ2K reaches the same conclusion through independently coded code-blocks and
a high-throughput cleanup representation. JPEG XS takes the other principled
route: groups of four coefficients use deliberately simple magnitude,
bit-plane, and sign representations instead of a powerful serial arithmetic
coder. These designs trade small amounts of format overhead or coding power
for bounded decoder state and regular execution.

Fastvid's four-state rANS mode already expresses multiple states, and fixed
block-pack mode already bounds blocks to 128 residuals. Rice payloads,
however, are one undelimited bitstream per tile. Their encoder can compute
code lengths in parallel and use a prefix scan, but the decoder still needs
restart points or multiple normative lanes to find symbol boundaries.

### Two-pass output is normal, not a mutex problem

Variable-size parallel encoding naturally has two phases:

1. each unit computes or writes its size into private scratch;
2. an exclusive prefix sum assigns canonical output offsets;
3. units write directly into disjoint final ranges.

Work-efficient parallel scan has linear work and logarithmic span. It replaces
mutex-protected append/`collect` patterns with a deterministic layout that
also matches Fastvid's existing directory semantics. The same plan works on a
CPU pool and CUDA; only the scan implementation differs.

### Memory layout and divergence are codec concerns

CUDA guidance prioritizes coalesced global accesses and reuse through shared
memory. A codec should therefore keep planar samples and fixed-shape
metadata contiguous, bucket work by predictor/entropy kernel, and avoid
per-unit heap allocation. Mixing zero-run, Rice, block-pack, and rANS tiles
inside one warp creates data-dependent divergence; a compact mode-index pass
followed by homogeneous kernel queues is preferable.

Uniform work is as important as average work. Maximum and p95 bytes, samples,
and cycles per unit should be recorded because one very large or pathological
Rice tile can become a straggler even when mean occupancy is high.

## Fastvid serialization audit

At default 256x128 luma geometry, planar YUV 4:2:2 gives every plane the same
tile grid because chroma tiles are 128 pixels wide. Therefore:

- 1280x720 has `3 * 5 * 6 = 90` independent tiles;
- 1920x1080 has `3 * 8 * 9 = 216`;
- 3840x2160 has `3 * 15 * 17 = 765`.

A full luma tile still contains 32,768 samples. Clamp-gradient reconstruction
has left, above, and upper-left dependencies. Its theoretical diagonal
wavefront span is `width + height - 1 = 383` synchronization steps, but the
current scalar traversal and single Rice stream both have a 32,768-symbol
serial span. Chroma tiles contain at most 16,384 samples. Temporal prediction
is much friendlier: residual formation is sample-independent, although its
entropy payload remains serial.

The current directory solves frame-level placement but not intra-tile
parallelism. Smaller default tiles would increase task count, but
[research 0028](0028-tile-geometry-tradeoffs.md) shows that geometry is
content-sensitive and should not be fitted to the development corpus. The
more principled format direction is to separate:

- **access tiles**, which retain the current random-access and predictor
  boundary;
- **execution shards**, which add bounded entropy/predictor restart or lane
  structure inside a tile.

## Design rules for a CUDA-oriented Fastvid format

1. Keep the scalar normative mapping and exact CPU/GPU agreement.
2. Preserve independently indexed access tiles; never introduce a
   frame-global adaptive entropy state.
3. Define a maximum serial span in samples, not just a tile size.
4. Treat entropy lane/restart positions as syntax. Charge every size, offset,
   terminal state, and padding bit.
5. Prefer fixed-size execution shards plus a short tail. Do not tune shard
   size on corpus v2 alone.
6. Use size → exclusive scan → disjoint write for canonical output assembly.
7. Store planar source/reconstruction data and structure-of-arrays metadata
   so adjacent lanes access adjacent words.
8. Bucket execution by predictor and entropy mode to limit warp divergence.
9. Report p95/max work per shard and scratch/output memory amplification,
   not only aggregate MP/s.
10. Keep temporal dependency depth and single-frame access separate from
    intra-frame hardware parallelism.

## Candidate exploration order

1. **Analytical restart budget:** measure the directory/padding lower bound
   for 256–4096-symbol execution shards without changing prediction.
2. **Multi-lane Rice model:** compare explicit per-lane lengths against a
   GDeflate-like defined refill schedule. Reject any model whose complete-byte
   cost erases its compression advantage over 128-symbol block pack.
3. **Independent predictor shards:** measure compression and boundary artifacts
   for row bands or blocks. This is a format experiment and requires the
   standard corpus plus a frozen validation corpus if a default size is
   selected.
4. **CUDA prototype:** only after the format has a bounded serial span. Measure
   kernels, transfers, launch overhead, and end-to-end codec time separately.

## Relevant experiments

- [EXP-0099](../experiments/EXP-0099-interleaved-rice-tile-pairs.md)
- [EXP-0100](../experiments/EXP-0100-parallel-serialization-budget.md)
- [EXP-0102](../experiments/EXP-0102-four-lane-rice-shard-model.md)
- [EXP-0103](../experiments/EXP-0103-independent-predictor-bands.md)
- [EXP-0104](../experiments/EXP-0104-predictor-band-height-ladder.md)
