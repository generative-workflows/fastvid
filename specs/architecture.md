# Prototype architecture and invariants

## Data flow

```
Frame + optional reconstructed reference → plane/tile partition
      → tile-local spatial/temporal predictor selection → residual → quantizer
      → per-tile zero-run/Rice/order-0 selection → canonical directory + header
```

Decoding reverses the flow. Each tile owns its predictor state and payload;
workers never depend on adjacent tiles. The final plane assembly is a
non-overlapping copy.

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
  and canonical initial rANS state.
- A zero run cannot exceed the remaining samples in its tile.
- Rice padding is zero and every entropy payload is consumed exactly.
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
- Version-3 order-0 entropy is scalar and deliberately occupies the
  compression frontier; its decode and random-access cost is not suitable for
  the balanced line without further optimization.
