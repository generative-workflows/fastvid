# Rolling reconstruction state for spatial prediction

## Basis

This note applies the cache-layout findings in
[research 0012](0012-simd-cache-profiling.md) to Fastvid's native high-bit
encoder. It is a source-derived implementation analysis; it does not claim
measured cache misses because hardware performance counters are unavailable on
the current host.

## Current working set

For each 256x128 luma tile, high-bit residual generation allocates:

- 65,536 bytes for a full `u16` reconstructed tile;
- 131,072 bytes for the `u32` folded-residual tile;
- the shared 10-bit quantizer table and source rows.

Spatial Paeth prediction only depends on the reconstructed left, above, and
upper-left samples. A single `width`-element row is sufficient:

1. before replacement, `row[x]` is the above sample;
2. a scalar retains the old above sample as upper-left for the next column;
3. another scalar retains the newly reconstructed left sample;
4. `row[x]` is replaced by the current reconstructed sample.

For a 256-pixel tile this reduces reconstruction state from 65,536 bytes to
512 bytes. It also removes zero-initialization proportional to tile area.

Temporal prediction has no reconstructed-sample dependency during encoding.
The current generic high-bit loop nevertheless allocates and writes the full
reconstruction tile and performs an unused multiply/clamp per sample. A
separate temporal loop can omit all of that work while retaining identical
folded residuals.

Neither change alters prediction boundaries, quantization, entropy selection,
or stream syntax. Exact baseline/candidate byte comparison is therefore the
primary correctness oracle.

## Relevant experiments

- [EXP-0032](../experiments/EXP-0032-rolling-high-bit-reconstruction.md)

