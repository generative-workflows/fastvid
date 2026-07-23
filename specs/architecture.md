# Prototype architecture and invariants

## Data flow

```
Frame → plane/tile partition → Paeth residual → quantizer → zero-run tokens
      → per-tile payloads → canonical directory + header
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
- Varints are canonical and bounded to `u32`.
- A zero run cannot exceed the remaining samples in its tile.
- Lossy prediction uses reconstructed neighbors on both encoder and decoder.
- Quality 100 selects step one and must round-trip exactly.

## Current limitations

- One frame per file; no temporal prediction or stream/container layer.
- 8-bit Gray and YUV 4:2:2 only.
- Thread creation and mutex-based job/result coordination are prototype
  mechanisms, not a production worker pool.
- No checksum, SSIM/MS-SSIM, rate-distortion harness, SIMD, or Aeneas bridge.
- Entropy coding is specialized for zero runs and has not been validated on a
  representative corpus.

