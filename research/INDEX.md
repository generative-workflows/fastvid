# Research index

Only openly readable sources with implementation-compatible terms are used as
design inputs. A literature note is not evidence that an algorithm is
patent-free; potentially encumbered techniques require a separate clearance
before entering the format.

| ID | Source | Relevance | Status |
|---|---|---|---|
| [0001](0001-ffv1.md) | IETF RFC 9043, FFV1 | Slices, intra coding, archival/intermediate use | Reviewed |
| [0002](0002-png-predictors.md) | W3C PNG filter specification | Simple causal spatial prediction | Reviewed |
| [0003](0003-ssim.md) | Wang et al., SSIM | Perceptual-quality measurement | Reviewed |
| [0004](0004-codec-evaluation.md) | IETF RFC 8761 and Xiph test media | Reproducible codec evaluation | Reviewed |
| [0005](0005-adaptive-rice-coding.md) | Rice/Plaunt, RFC 9043, RFC 9639 | Adaptive coding of predictor residuals | Reviewed |
| [0006](0006-standard-evaluation-methodology.md) | RFC 8761, AOM CTC, Xiph methodology | Standard corpus and evaluation protocol | Reviewed |
| [0007](0007-temporal-dpcm-gating.md) | Rice/Plaunt and CCSDS | Previous-frame DPCM and activity gating | Reviewed |
| [0008](0008-block-local-inter-intra-selection.md) | RFC 6386 and AV1 specification | Block-local spatial/temporal prediction selection | Reviewed |
| [0009](0009-corpus-v2-diversity.md) | Wikimedia CC0 TIFFs, Xiph, RFC 8761 | Camera/procedural corpus coverage and licensing | Reviewed |
| [0010](0010-single-frame-random-access.md) | RFC 6386, RFC 8761, AV2 CTC | Single-frame access cost and GOP preroll | Reviewed |
| [0011](0011-openapv.md) | Academy Software Foundation OpenAPV | Royalty-free professional intra-codec architecture and comparison target | Reviewed |
| [0012](0012-simd-cache-profiling.md) | Rust `std::arch`/`std::simd`, Cachegrind | Safe SIMD strategy, cache profiling, and current hot-path candidates | Reviewed |
| [0013](0013-high-bit-depth-codec-design.md) | OpenAPV, FFV1 | Native 10/12/16-bit formats, arithmetic bounds, and compatibility | Reviewed |
| [0014](0014-sampling-and-high-bit-quantization.md) | Samply, Linux perf security, Rust `Vec` | Statistical sampling limits and contiguous lookup-table guarantees | Reviewed |
| [0015](0015-openapv-matched-comparison.md) | OpenAPV v0.3.0.0 source | Preset defaults, application clocks, tiling, and SIMD build behavior | Reviewed |
| [0018](0018-modern-perceptual-metrics.md) | Recent SSIM descendants, FUNQUE+, DISTS, ColorVideoVDP | Fast/slow metric tiers, texture, temporal, color, and HDR quality | Reviewed |
| [0019](0019-modern-integer-entropy-kernels.md) | Stream VByte, exact reciprocal arithmetic, recent Rice LUT work | Control/data separation, lookup tables, and exact integer kernels | Reviewed |
| [0020](0020-modern-parallel-codec-kernels.md) | vecSZ, JPEG XL, AV1 | Dependency removal, SIMD structure, cache-aware block architecture | Reviewed |
| [0021](0021-rayon-work-stealing.md) | Rayon 1.12 documentation and source | Persistent work-stealing pools and ordered parallel tile collection | Reviewed |
| [0022](0022-parking-lot-mutex.md) | `parking_lot` 0.12.5 documentation and source | Adaptive userspace mutex fast path for tile-output coordination | Reviewed |
| [0023](0023-forward-citation-space-savings.md) | Recent predictor papers, WebP lossless, ANS/FSE, and codec benchmarks | Forward-citation pass for bounded residuals, block predictors, and exact byte savings | Reviewed |
| [0024](0024-finite-block-ans-entropy-models.md) | ANS/FSE, interleaved coders, and finite-block source coding | Complete-byte table-overhead model for tile residual entropy | Reviewed |
| [0025](0025-context-conditioned-residual-entropy.md) | Recent JPEG XL, conditional residual, and hierarchical probability work | Charged causal-context models for residual scale and cross-channel structure | Reviewed |
| [0026](0026-paeth-data-dependency-kernel.md) | stb_image, JPEG XL, PNG | Byte-identical Paeth data dependencies and fixed-predictor speed paths | Reviewed |
| [0027](0027-streaming-rice-parameter-selection.md) | Recent Rice-parameter analysis, CharLS, FLAC | Sparse parameter estimation and one-pass residual entropy writing | Reviewed |
| [0028](0028-tile-geometry-tradeoffs.md) | JPEG XL, APV RFC 9924, OpenAPV | Rectangular tile rate, throughput, cache, parallelism, and access tradeoffs | Reviewed |
| [0029](0029-block-translational-inter-prediction.md) | AV1 overview/specification and rav1e | Integer block-motion potential with bounded GOP dependencies | Reviewed |
| [0030](0030-entropy-decode-consumer-fusion.md) | ryg_rans, interleaved entropy coders, FSE | Direct entropy consumption and the multi-state SIMD format boundary | Reviewed |
| [0031](0031-modern-simd-rans-implementation.md) | htscodecs SIMD rANS, ryg_rans | Wider-state vector kernels, gather regressions, and Zen 4 limits | Reviewed |
| [0032](0032-chroma-from-luma-prediction.md) | AV1 CfL paper, specification, and SVT-AV1 | Charged tile-local cross-plane chroma prediction | Reviewed |
| [0033](0033-reversible-squeeze-transform.md) | JPEG XL 2025 overview and libjxl v0.11.2 | Reversible lifting, frequency separation, and SIMD dependency layout | Reviewed |
| [0034](0034-block-bitpacking-kernels.md) | FastPFOR/SIMD-BP128, Revec, SFVInt | Charged 128-symbol bit packing, specialized widths, and SIMD/BMI dispatch limits | Reviewed |
| [0035](0035-runtime-invariant-integer-division.md) | Optimal reciprocal bounds, libdivide, `strength_reduce` | Exact runtime-invariant quantization without dependent table loads | Reviewed |
| [0036](0036-independent-chain-software-pipelining.md) | AMD/Intel optimization manuals, LLVM pipeliner, Cimple | Interleave independent causal tile chains for scalar ILP | Reviewed |
| [0037](0037-parallel-hardware-friendly-codecs.md) | HTJ2K, JPEG XL/XS, GDeflate, CUDA scan | Independent work units, bounded serial span, entropy lanes, and two-pass output | Reviewed |
| [0038](0038-lossless-wavefront-scheduling.md) | Recent lossless wavefront codecs, PILC, DietGPU | Exact causal scheduling, raster staging, and parallel entropy handoff | Reviewed |
| [0039](0039-parallel-rice-bitstream-hardware.md) | 2024 parallel Bayer FPGA codec and OMLS license audit | Fixed-width parallel Rice packing, bounded unary output, and incompatible source boundary | Reviewed |
| [0040](0040-edge-gpu-predictive-compression.md) | Ferraz et al., parallel CCSDS-123 on Jetson GPUs | Causal-kernel isolation, heterogeneous entropy selection, and serial-output limits | Reviewed |
