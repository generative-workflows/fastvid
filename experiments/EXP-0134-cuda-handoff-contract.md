# EXP-0134 — CUDA handoff contract

Status: **ACCEPTED**

## Classification

**Parallel-hardware exploration / specification** — turn version 5's
low-serialization intent into an implementable first CUDA pipeline before
migration to a GPU node.

## Hypothesis

Version 5 can be emitted canonically on a GPU without a shared append,
single-thread concatenation, or format change because every entropy candidate
has an exact local byte count and every selected body has a bounded length.
Its full-tile predictor also admits two competing GPU schedules—antidiagonal
wavefront and interleaved independent chains—that produce the same canonical
raster residual field.

A useful handoff must specify buffers, kernel boundaries, offset equations,
serial spans, correctness oracles, and measurements rather than merely say
"use CUDA."

## Modification

Research recent GPU variable-output practice and add a concrete version-5
CUDA contract to `specs/architecture.md` and stage-level GPU requirements to
`EVALUATION_METHODOLOGY.md`:

1. stage full-tile residuals in canonical raster storage;
2. count zero-run, four-lane Rice, and fixed-block candidates per shard;
3. select one exact body size and mode;
4. exclusive-scan canonical shard record sizes;
5. reduce shard sizes per tile;
6. emit selected records directly to disjoint output spans;
7. write fixed-stride tile directory entries and the header.

Define both predictor schedules and require them to match the scalar Rust
oracle exactly. Define stage-level timing, memory, divergence, and span
measurements for the GPU node.

No Rust encoder, decoder, format, frontier slot, or default changes.

## Test

Audit the current v5 source and format invariants:

- shard record size is exactly `3 + selected_body_bytes`;
- selected bodies fit the existing `u16` shard length;
- Rice lane offsets derive from exact lane byte counts;
- tile payloads are contiguous canonical shard sequences;
- tile offsets and lengths derive only from the ordered shard-size scan;
- directory entries are fixed-width and independently writable;
- the decoder rejects non-canonical offsets, lengths, padding, or trailing
  data;
- scalar and paired staging already agree on folded raster residuals.

Require the future CUDA implementation to pass every existing v5
round-trip/malformed-stream test, reproduce the q90 control SHA-256, and
compare every corpus stream byte with the Rust oracle before performance is
considered.

## Result

The audit passes. The existing syntax needs no extra offset table:

```text
record_size[i] = 3 + body_size[i]
record_offset  = exclusive_sum(record_size)
tile_offset    = payload_start + record_offset[first_shard]
tile_length    = sum(record_size for the tile)
stream_length  = payload_start + sum(record_size)
```

Integer scans are deterministic, and each selected record interval is
disjoint. One fixed directory entry per access tile remains sufficient.
Current checked CPU accumulation maps to 64-bit device counts followed by
explicit `u16` shard-body and `u32` tile-body representability checks.

The important remaining serial boundary is prediction, not output assembly.
A 256x128 raster chain has 32,768 sample steps; an antidiagonal execution has
383 synchronization steps with up to 128 independent samples per step.
Interleaved independent chains are the competing lower-synchronization
schedule. Both preserve the current format by scattering into the same
raster residual array before entropy selection.

The resulting first-node experiment matrix is:

| Stage | Initial mapping | Competing mapping |
|---|---|---|
| Predictor | one block/tile, antidiagonal wavefront | independent tile chains per lane/warp |
| Candidate count | one block/4,096-symbol shard | homogeneous mode queues |
| Placement | device-wide exclusive scan | hierarchical tile + shard scan |
| Emission | one block/selected shard, direct span | mode-specialized kernels |
| Directory | one thread/tile | fused with final placement kernel |

This is a design/readiness result, not CUDA performance evidence. The
authoritative implementation and timing work begins on the GPU node.

## Decision

Accept the contract as the CUDA starting point. Keep both predictor schedules
alive until measured; do not assume wavefront wins merely because its
dependency depth is smaller. Preserve v5's 4,096-symbol shards and four Rice
lanes for the first prototype so format engineering and kernel engineering
are not confounded.

The first GPU milestone is byte-identical encode/decode with stage timings.
Only then explore bounded-unary Rice escape, different shard sizes, kernel
fusion, CUDA Graphs, or device-resident multi-frame pipelines.

## References

- [Research 0038](../research/0038-lossless-wavefront-scheduling.md)
- [Research 0039](../research/0039-parallel-rice-bitstream-hardware.md)
- [Research 0040](../research/0040-edge-gpu-predictive-compression.md)
- [Research 0042](../research/0042-gpu-variable-output-assembly.md)
- [EXP-0100](EXP-0100-parallel-serialization-budget.md)
- [EXP-0105](EXP-0105-predictor-wavefront-model.md)
- [EXP-0110](EXP-0110-full-tile-bounded-shards.md)
- [EXP-0129](EXP-0129-interleaved-full-tile-predictors.md)
