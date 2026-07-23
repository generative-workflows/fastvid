# Modern parallel codec kernels and dependency removal

## Citation trail

This note follows newer codec work from the PNG/predictive, video-codec, and
high-bit architecture sources in [research 0002](0002-png-predictors.md),
[research 0008](0008-block-local-inter-intra-selection.md), and
[research 0013](0013-high-bit-depth-codec-design.md).

## Sources

- Dube et al., [*SIMD Lossy Compression for Scientific Data*][vecsz-paper],
  2022, and its [research repository][vecsz-code].
- Rhatushnyak et al., [*Committee Draft of JPEG XL Image Coding
  System*][jxl-2019], 2019.
- Sneyers et al., [*The JPEG XL Image Coding System: History, Features,
  Coding Tools, Design Rationale, and Future*][jxl-2025], 2025.
- Han et al., [*A Technical Overview of AV1*][av1], 2020/2021.

[vecsz-paper]: https://arxiv.org/abs/2201.04614
[vecsz-code]: https://github.com/szcompressor/vecSZ
[jxl-2019]: https://arxiv.org/abs/1908.03565
[jxl-2025]: https://arxiv.org/abs/2506.05987
[av1]: https://arxiv.org/abs/2008.06091

## Remove dependencies before adding intrinsics

vecSZ starts from the observation that a conventional prediction/quantization
loop has loop-carried dependencies that inhibit SIMD. Its dual-quantization
formulation reorganizes the work so independent blocks can be vectorized, then
selects block size and vector width jointly. On the paper's AVX2 and AVX-512
hosts, the vectorized prediction/quantization kernel achieved about 1.5x over
its already reorganized scalar baseline on average, with larger gains against
the older dependent implementation. Higher-dimensional traversal suffered
more cache misses and smaller gains.

The direct Fastvid lesson is structural:

1. preserve a simple scalar normative mapping;
2. isolate independent sample work from causal reconstruction;
3. vectorize the independent phase;
4. tune block size against cache behavior, not SIMD width alone.

Fastvid's spatial Paeth reconstruction remains causal, but quantization,
folding, histogram construction in lane-local accumulators, and temporal
residual formation may be separated into independently vectorizable blocks.
The vecSZ numerical transform is for floating-point scientific arrays and
must not be transplanted into Fastvid's image format. Its repository does not
display a recognized license in the reviewed project page, so code copying is
not an implementation option without clarification.

## Parallelism is a format property as well as an implementation property

JPEG XL explicitly targets fast parallel encode/decode configurations while
supporting lossless coding, animation, high bit depth, wide gamut, and HDR.
AV1 reports substantial rate reduction while retaining hardware-feasible
tools. Both architectures achieve parallelism through bounded blocks/tiles
and carefully specified dependencies rather than by expecting SIMD to solve a
globally serial algorithm.

For Fastvid this reinforces:

- tiles must remain independently decodable, with bounded state and explicit
  edge behavior;
- single-frame and tile-level access costs must be measured alongside
  sequential throughput;
- a more complex predictor is acceptable only if its control overhead,
  dependency depth, and worst-case working set remain bounded;
- high-bit paths should share the same dependency graph as 8-bit paths even
  when lane width and accumulators differ.

The published JPEG XL and AV1 headline rate improvements are not comparable to
Fastvid measurements: their content, operating points, quality metrics, and
codec goals differ. They are architecture references, not benchmark rows.

## Candidate kernels, ordered by evidence

1. **Entropy finalization:** current sampling and Cachegrind results identify
   this as the largest encode/cache hotspot. Test cache-resident exact tables
   and separated control/data writes before format changes.
2. **Reconstruction:** retain the accepted rolling-row state, then inspect
   whether independent chroma pairs or temporal rows auto-vectorize.
3. **Histogram construction:** test lane-local small histograms or blocked
   reduction only if PMU data shows write conflicts or cache pressure; the
   extra reduction pass can lose.
4. **Explicit SIMD:** add target-specific kernels only after safe scalar loop
   reshaping fails and the resulting assembly proves missing vectorization.

Every candidate remains subject to exact-stream comparison, malformed-input
tests, high-bit overflow checks, corpus timing, and per-sample compression and
quality gates.

## Relevant experiments

- [EXP-0022: LLVM vectorization audit](../experiments/EXP-0022-llvm-vectorization-audit.md)
- [EXP-0032: rolling high-bit reconstruction](../experiments/EXP-0032-rolling-high-bit-reconstruction.md)
- [EXP-0034: perf, Samply, and Cachegrind profile](../experiments/EXP-0034-perf-samply-cache-profile.md)
