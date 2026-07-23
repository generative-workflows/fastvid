# OpenAPV as an intermediate-codec reference

## Source

- Academy Software Foundation, [OpenAPV repository][openapv].
- OpenAPV version `v0.3.0.0`, released 2026-06-29.
- Source archive SHA-256:
  `dc5cd1618a07e8b340e12562cae37d612b3a1467ee80d986c477165ae602a37e`.
- License: BSD-3-Clause.

[openapv]: https://github.com/AcademySoftwareFoundation/openapv

## Relevant design

OpenAPV is the reference implementation of the royalty-free APV professional
video codec. It is a useful performance and capability target for Fastvid:

- intra-frame-only coding and frame tiling make random frame and tile access
  first-class operations;
- the codec supports 4:2:2, 4:4:4, 4:4:4:4, and monochrome profiles at 10 and
  12 bits, with higher bit depths described by the APV family;
- it uses block transforms and lightweight entropy coding without pixel-domain
  prediction;
- it uses tile-level threading;
- x86 builds include SSE4.1 and AVX2 sources, ARM builds include NEON sources,
  and runtime function tables select optimized SAD, difference, transform,
  inverse-transform, quantization, and dequantization kernels;
- HDR and auxiliary pictures such as alpha are explicit capabilities.

OpenAPV therefore sets a stronger target than merely beating an old delivery
codec: high-bit-depth fidelity, direct frame access, tile parallelism, and
architecture-specific hot kernels are central to its design.

## Comparison constraints

Fastvid currently accepts planar 8-bit 4:2:2 while OpenAPV's application
accepts 10-bit-or-greater YCbCr. A direct speed/ratio comparison would therefore
confound sample precision, raw byte count, and quality. The comparison protocol
must:

1. derive a pinned 10-bit 4:2:2 corpus from the same source frames;
2. exclude format-conversion and file-I/O time;
3. report source samples/s and pixels/s as well as raw bytes/s;
4. match reconstruction quality, not nominal quality/QP controls;
5. compare intra-only Fastvid (GOP 1) with OpenAPV;
6. report encoded bits/pixel and bitrate at the sample frame rate;
7. record CPU feature dispatch, thread count, version, and command line.

Until Fastvid has a high-bit-depth profile, OpenAPV is an architectural and
throughput target rather than a headline apples-to-apples result.

## Independently derived implications

- Keep frame and tile independence measurable.
- Separate portable orchestration from narrow hot kernels.
- Prefer safe loop forms that LLVM can auto-vectorize before introducing
  architecture-specific unsafe intrinsics.
- Treat 10/12-bit, HDR metadata, and alpha/auxiliary planes as format goals,
  not as conversions into the existing 8-bit path.
- Reuse workers across frames if profiling confirms thread creation is material.

No OpenAPV source code is incorporated into Fastvid.

## Relevant experiments

- [EXP-0010](../experiments/EXP-0010-fast-feedback-loop.md) defines a rapid
  Fastvid iteration gate.
- [EXP-0011](../experiments/EXP-0011-parallel-map-contention.md) measures a
  threading/layout optimization motivated by the architectural comparison.

