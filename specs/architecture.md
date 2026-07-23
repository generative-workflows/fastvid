# Prototype architecture and invariants

## Data flow

```
Frame → plane/tile partition → Paeth residual → quantizer
      → per-tile zero-run/Rice selection → canonical directory + header
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
  the zigzag image of the codec's signed 8-bit residual range.
- A zero run cannot exceed the remaining samples in its tile.
- Rice padding is zero and every entropy payload is consumed exactly.
- Lossy prediction uses reconstructed neighbors on both encoder and decoder.
- Quality 100 selects step one and must round-trip exactly.

## Current limitations

- One frame per file; no temporal prediction or stream/container layer.
- 8-bit Gray and YUV 4:2:2 only.
- Thread creation and mutex-based job/result coordination are prototype
  mechanisms, not a production worker pool.
- No checksum, MS-SSIM, rate-distortion corpus harness, SIMD, or Aeneas bridge.
- Block SSIM is available, but entropy coding has not yet been validated on a
  representative natural-image corpus.
