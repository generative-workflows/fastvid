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

EXP-0108 implemented that model as diagnostic high-bit version 4. It improved
scalar decode and independent-tile access, but was rejected as a frontier
format because its actual block-pack-relative q90 rate regressed 3.846% and
its exhaustive scalar encoder reached only about 10–11 MP/s. The next branch
retained bounded raster entropy shards but removed predictor-band restarts by
using the full-tile wavefront dependency graph.

EXP-0110 implements that branch as experimental high-bit version 5. Full-tile
clamp-gradient prediction preserves version 2's residual field, while
4,096-symbol raster shards independently select zero-run, four-lane Rice, or
128-symbol fixed-block bodies. Compact 16-bit shard lengths are sufficient
because fixed block bounds every selected body below 64 KiB. On the native
q90 screen this costs 0.801% aggregate bytes, improves scalar full decode
26.1% geometrically, and improves independent-tile access 33.1%. Its current
scalar encoder uses exact candidate costs to construct only the selected
zero-run/fixed-block body and one pre-sized Rice body. Exact Rice search and
CPU scheduling work has raised it from roughly 12 MP/s to 42–67 MP/s on the
native screen without changing bytes. It remains diagnostic rather than
speed-competitive. The format maps to a two-pass
classify/scan/disjoint-write CUDA pipeline without shared append
serialization.

## CUDA version-5 handoff

The first CUDA implementation keeps the version-5 stream unchanged. It uses
the following device-visible arrays:

- planar source and reconstruction samples (`u16`);
- canonical raster folded residuals (`u32`);
- one exact selection record per 4,096-symbol shard containing mode, selected
  body size, Rice parameter/lane sizes, and fixed-block metadata;
- 64-bit shard record sizes and exclusive offsets;
- 64-bit per-tile payload sizes/offsets before checked narrowing; and
- one pre-sized output byte buffer.

For shard `i`, `record_size[i] = 3 + body_size[i]`, accounting for the mode
byte and `u16` body length. Flatten shards in canonical plane/tile/raster
order and apply an exclusive integer scan:

```text
record_offset[i] = exclusive_sum(record_size)[i]
absolute_offset  = payload_start + record_offset[i]
```

A per-tile reduction supplies the directory length; the first shard offset
supplies the directory payload offset. The final scan value plus final record
size supplies the allocation length. Selected-mode kernels write directly to
these disjoint intervals. A fixed-stride kernel writes directory entries.
Header emission is constant work. No kernel appends through an atomic cursor
or mutex, and no concatenation pass copies partial bodies.

Prediction has a separate scheduling choice. A 256x128 full-tile raster chain
contains 32,768 dependent samples. Antidiagonal execution reduces its graph
depth to 383 synchronized steps, with at most 128 samples on one diagonal.
The initial implementation compares:

1. one block per tile advancing antidiagonals and scattering folded residuals
   to raster locations; and
2. independent scalar tile chains assigned across lanes/warps, analogous to
   EXP-0129's CPU interleaving.

Both schedules must reproduce the scalar Rust residual field exactly before
entropy counting. Entropy candidate counting, scan, emission, and directory
write remain separate kernels in the first correct prototype. Fusion is
considered only after stage timings expose launch or memory traffic as a
material cost.

EXP-0138 measured the corresponding decoder reconstruction schedules on an
NVIDIA L40. Both were byte-exact, but the one-thread-per-tile scalar chains
were 10.45x slower than antidiagonal wavefront on the real-world 4K q90 row.
Wavefront is therefore the default CUDA reconstruction schedule; scalar tile
chains remain a diagnostic control rather than a performance candidate.

The first encoder now implements this pipeline while preserving the Rust v5
stream byte-for-byte. It performs exact parallel candidate analysis, transfers
only compact selection records for the canonical host scan and directory, and
writes pre-sized device output intervals. Four warps emit the four Rice lanes.
Fixed-block shards are compacted from the already transferred selection data
and use one warp per 128-symbol block, eliminating serial per-block bit
packing without changing the format. The host scan remains an intentional
baseline and a future device-side assembly experiment.

GPU evaluation records transfers, predictor, candidate selection, scan,
emission, directory, and end-to-end time independently; it also records peak
device memory, achieved bandwidth, active warps, warp/branch efficiency,
predictor steps, and maximum selected lane/codeword output. The complete
stream must match the Rust oracle byte-for-byte over the standard corpus.

Research and rationale are in
[`research/0042-gpu-variable-output-assembly.md`](../research/0042-gpu-variable-output-assembly.md)
and [EXP-0134](../experiments/EXP-0134-cuda-handoff-contract.md).

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
