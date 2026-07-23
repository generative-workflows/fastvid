# Fastvid experimental bitstream, version 0

Status: **experimental; incompatible changes are allowed**

All multibyte integers are unsigned little-endian. A file contains one frame.
The frame-rate is rational and metadata-only.

## File header (32 bytes)

| Offset | Size | Meaning |
|---:|---:|---|
| 0 | 4 | ASCII `FVID` |
| 4 | 1 | version = 0 |
| 5 | 1 | pixel format: 0 = Gray8, 1 = YUV422p8 |
| 6 | 1 | quality, 1…100 |
| 7 | 1 | reserved, zero |
| 8 | 4 | luma width |
| 12 | 4 | luma height |
| 16 | 2 | nominal luma tile width |
| 18 | 2 | nominal luma tile height |
| 20 | 4 | frame-rate numerator, nonzero |
| 24 | 4 | frame-rate denominator, nonzero |
| 28 | 4 | tile count |

Gray8 has one plane of `width × height`. YUV422p8 has Y of `width × height`
followed by Cb and Cr of `ceil(width / 2) × height`.

## Tile directory

Each entry is 32 bytes:

| Offset | Size | Meaning |
|---:|---:|---|
| 0 | 1 | plane index |
| 1 | 1 | entropy mode: 0 = zero-run varints; 1…9 = Rice parameter 0…8 |
| 2 | 2 | reserved, zero |
| 4 | 4 | plane-local x |
| 8 | 4 | plane-local y |
| 12 | 4 | width |
| 16 | 4 | height |
| 20 | 8 | payload offset from file start |
| 28 | 4 | payload byte length |

Entries cover each plane exactly once without overlap. Chroma tile width is
`ceil(nominal_luma_tile_width / 2)`. Every tile is independently decodable.
Entries are canonical: planes occur in Y/Cb/Cr order and tiles within a plane
occur in row-major order. Payloads are contiguous in directory order with no
padding or trailing bytes.

## Tile payload

Samples are processed in raster order. For each sample, predict from already
reconstructed samples in the same tile with Paeth:

```
p = left + above - upper_left
predict = argmin([left, above, upper_left], |p - candidate|)
```

Ties prefer left, then above, then upper-left. Missing neighbors are zero.

Quality selects quantization step `q = 1 + floor((100-quality) / 5)`. The
signed residual `sample - predict` is divided by `q` with symmetric
round-to-nearest, ties away from zero. Prediction state uses the clamped
reconstruction `predict + quantized_residual*q`.

The quantized signed residual is zigzag mapped:

```
0 → 0, -1 → 1, 1 → 2, -2 → 3, 2 → 4, ...
```

Entropy mode zero represents the residual sequence with tokens:

```
token = 2*(run_length-1)              for a run of zero residuals
token = 2*zigzag(nonzero_residual)-1  for one nonzero residual
```

Tokens are encoded as unsigned LEB128. A decoder rejects non-canonical,
overflowing, truncated, overlong-run, or trailing payload bytes.

Entropy modes one through nine encode every zigzag-mapped residual with Rice
parameter `k = mode - 1`. A value `n` is split into quotient `n >> k` and the
`k` low remainder bits. The quotient is that many zero bits followed by a one;
the remainder follows least-significant bit first. Bits fill bytes from least
to most significant. The last byte is zero-padded. A decoder rejects truncated
codes, folded residuals above 510, nonzero padding, and trailing bytes.

The encoder computes the exact byte length for zero-run mode and Rice
parameters 0…8 independently per tile. It selects the shortest representation,
preferring zero-run mode and then the lower Rice parameter on ties. The mode
occupies a directory byte that was reserved in the original prototype, so mode
selection adds no payload bytes.
