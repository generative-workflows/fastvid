# Prototype architecture and invariants

## Data flow

```
Frame + optional reconstructed reference → plane/tile partition
      → tile-local spatial/temporal predictor selection → residual → quantizer
      → per-tile zero-run/Rice/block-pack/order-0 selection
      → canonical directory + header
```

Decoding reverses the flow. Each tile owns its predictor state and payload;
workers never depend on adjacent tiles. The final plane assembly is a
non-overlapping copy.

## Parallel-hardware mapping

The current format's access tile is also its smallest independently indexed
execution unit. At default 256x128 luma geometry this yields 90 tiles at
1280x720, 216 at 1920x1080, and 765 at 3840x2160 for planar YUV 4:2:2. A full
luma tile nevertheless contains 32,768 samples:

- spatial clamp-gradient reconstruction is causal through reconstructed left,
  above, and upper-left samples;
- a Rice payload has one variable-length decoder state spanning the tile;
- zero-run payloads are also sequential within their runs;
- fixed block pack bounds entropy decisions to 128 residuals, but the
  predictor remains tile-causal;
- temporal residual formation is sample-independent, although the selected
  entropy payload may still serialize it.

Thus tile count alone is not the parallelism contract. The evaluation
methodology separately records access units, execution shards, predictor
dependency span, and entropy-state span.

A future parallel-oriented format version should retain access tiles while
adding independently located execution shards or normative entropy lanes
inside them:

```text
classify/count each shard in parallel
              ↓
exclusive scan of canonical shard sizes
              ↓
write disjoint payload ranges in parallel
              ↓
write canonical tile/shard directory
```

No worker appends through a shared output mutex. Source, reconstruction, and
per-mode metadata remain planar/contiguous; implementations may compact shard
indices into homogeneous predictor/entropy queues to avoid SIMD/warp
divergence. The scalar Rust mapping remains normative and must agree exactly
with future CPU SIMD and CUDA kernels.

The leading measured format model is EXP-0107: independent 64-row predictor
bands retain raster residual order, split entropy at 4,096 symbols, and use
four byte-aligned Rice lanes where Rice wins. It bounds predictor units to
16,384 samples and entropy states to 4,096 symbols for +1.727% aggregate
modeled high-bit q90 bytes. This is accepted design evidence, not yet
normative syntax or an implemented stream version.

EXP-0108 implemented that model as diagnostic high-bit version 4. It improved
scalar decode and independent-tile access, but was rejected as a frontier
format because its actual block-pack-relative q90 rate regressed 3.846% and
its exhaustive scalar encoder reached only about 10–11 MP/s. The next branch
retains bounded raster entropy shards but removes predictor-band restarts by
using the full-tile wavefront dependency graph.

Research and quantitative gates are in
[`research/0037-parallel-hardware-friendly-codecs.md`](../research/0037-parallel-hardware-friendly-codecs.md)
and [`EVALUATION_METHODOLOGY.md`](../EVALUATION_METHODOLOGY.md).

## Core invariants

- Rust forbids unsafe code at the crate level.
- Dimensions, plane sizes, directory sizes, offsets, and payload ends use
  checked arithmetic before allocation or slicing.
- A decoded stream has canonical plane/raster tile order, contiguous
  payloads, exact coverage, and no trailing bytes.
- Varints are canonical and bounded to `u32`; Rice residuals are bounded to
  the zigzag image of the signaled sample-depth residual range.
- Version-3 order-0 tables have strictly increasing bounded symbols, positive
  frequencies summing to a power of two, exact sample and byte consumption,
  and canonical initial rANS state. Interleaved mode assigns sample `i` to
  state `i mod 4` and requires all four states to finish canonically.
- A zero run cannot exceed the remaining samples in its tile.
- Rice and fixed-block padding is zero and every entropy payload is consumed
  exactly. High-bit fixed blocks contain at most 128 folded residuals and
  signal one bounded bit width per byte-aligned block.
- Lossy prediction uses reconstructed neighbors on both encoder and decoder.
- Temporal prediction uses only the co-located sample in the preceding
  reconstructed frame; keyframes bound reference-chain length.
- Quality 100 selects step one and must round-trip exactly.

## Current limitations

- One frame per file; temporal frame dependencies exist, but there is not yet
  a sequence container or keyframe index.
- Version zero is legacy 8-bit, version one is legacy 10/12/16-bit, version
  two adds tile-local predictor modes to both layouts, and version three adds
  8-bit tile-local order-0 entropy as specified in `format-v3.md`.
- Thread creation and mutex-based job/result coordination are prototype
  mechanisms, not a production worker pool.
- No checksum, MS-SSIM, SIMD, or Aeneas bridge.
- Version-3 order-0 entropy has scalar one-state and four-state interleaved
  modes. The four-state safe-Rust kernel exposes instruction-level
  parallelism without architecture intrinsics; explicit SIMD remains future
  work.
