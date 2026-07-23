# Fastvid experimental bitstream, version 1

Status: **experimental; incompatible changes are allowed**

Version 1 adds explicit 8/10/12/16-bit samples while retaining independent
tiles and the version-zero predictor/entropy structure. All multibyte integers
are unsigned little-endian. A file contains one frame.

## File header (32 bytes)

| Offset | Size | Meaning |
|---:|---:|---|
| 0 | 4 | ASCII `FVID` |
| 4 | 1 | version = 1 |
| 5 | 1 | layout: 0 = Gray, 1 = planar YUV 4:2:2 |
| 6 | 1 | quality, 1…100 |
| 7 | 1 | bit depth minus 8: 0, 2, 4, or 8 |
| 8 | 4 | luma width |
| 12 | 4 | luma height |
| 16 | 2 | nominal luma tile width |
| 18 | 2 | nominal luma tile height |
| 20 | 4 | frame-rate numerator, nonzero |
| 24 | 4 | frame-rate denominator, nonzero |
| 28 | 4 | tile count |

Gray has one `width × height` plane. YUV 4:2:2 has Y of `width × height`
followed by Cb and Cr of `ceil(width/2) × height`.

Samples are unsigned integers in `[0, 2^bit_depth-1]`. The in-memory API uses
`u8` for 8-bit formats and `u16` for 10/12/16-bit formats. Raw high-bit-depth
interchange uses tightly packed little-endian `u16` words; bits above the
signaled depth must be zero.

## Tile directory

The 32-byte version-zero directory is retained. Entropy mode 0 is zero-run
varints. Modes 1…17 are Rice parameters 0…16. Prediction mode 0 is tile-local
spatial Paeth and mode 1 is the co-located reconstructed previous-frame sample.

Entries cover every plane exactly once in canonical plane/raster order.
Payloads are contiguous without padding or trailing bytes.

## Prediction and residual range

Paeth operates in signed 32-bit arithmetic and selects among unsigned sample
values. Missing tile neighbors are zero. Reconstruction is clamped to
`[0, 2^bit_depth-1]`.

For bit depth `b`, residuals are bounded by `±(2^b-1)` and zigzag values by
`2*(2^b-1)`. Decoders reject larger values before reconstruction.

Temporal prediction requires a reconstructed reference with identical
dimensions, layout, and bit depth.

## Quantization

Let:

```
base = 1 + floor((100-quality)/5)
step = 1 + ((base-1) << (bit_depth-8))
```

Residual division uses symmetric round-to-nearest with ties away from zero.
Quality 100 therefore has step one at every supported bit depth and must be
exact.

## Entropy payloads

Zigzag mapping, zero-run token syntax, unsigned canonical LEB128, and Rice bit
order are unchanged from version zero. Values and validation bounds widen to
the signaled bit depth.

For Rice parameter `k`, the encoder evaluates the exact bit count:

`sample_count*(k+1) + sum(folded_value >> k)`.

The shortest zero-run/Rice representation wins; ties prefer zero-run and then
the smaller Rice parameter. Decoders reject out-of-range folded values,
truncated codes, nonzero Rice padding, overlong runs, noncanonical varints, and
trailing bytes.

## Version-zero compatibility

A version-one decoder may accept version zero. Version zero implies bit depth
8, retains format values 0 = Gray8 and 1 = YUV422p8, requires header byte 7 to
be zero, limits Rice parameters to 0…8, and limits folded residuals to 510.

