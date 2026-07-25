# Fastvid experimental bitstream, version 2

Status: **experimental; incompatible changes are allowed**

Version 2 adds tile-local spatial-predictor selection to the 8-bit and
10/12/16-bit formats. It also defines high-bit entropy mode 18 for
fixed-width residual blocks. Header and directory lengths, quantization, and
tile order are unchanged.

## Header

The header is 32 bytes. Byte 4 is version 2. For 8-bit streams, byte 5 retains
the version-zero `PixelFormat` value and byte 7 is zero. For high-bit streams,
byte 5 is layout (0 Gray, 1 planar YUV 4:2:2) and byte 7 is bit depth minus
eight (2, 4, or 8). All remaining fields have the meanings and validation
rules in versions zero and one.

## Prediction modes

Directory byte 2 signals one predictor for the complete tile:

| Mode | Predictor |
|---:|---|
| 0 | Paeth |
| 1 | co-located sample in the preceding reconstructed frame |
| 2 | `floor((left + above) / 2)` |
| 3 | `clamp(left + above - upper_left, 0, sample_max)` |
| 4 | half gradient, defined below |

For half gradient:

```text
average = trunc((left + above) / 2)
prediction = clamp(
    average + trunc((average - upper_left) / 2),
    0,
    sample_max)
```

All division of a signed value truncates toward zero. The average numerator is
nonnegative, so its truncation equals floor division. Missing left, above, and
upper-left samples at tile boundaries are zero.

Every spatial predictor uses reconstructed samples from its own tile. The
encoder quantizes `source - prediction`, reconstructs and clamps the sample,
then advances predictor state. The decoder performs the same operations.
Tiles never read spatial state from adjacent tiles.

Mode 1 requires a reconstructed reference frame with identical dimensions,
layout, and sample depth. Spatial and temporal modes may be mixed within one
predicted frame. This does not increase GOP dependency depth.

## Predictor selection

The normative stream does not prescribe an encoder search. The experimental
reference encoder evaluates all applicable modes with the existing exact
zero-run/Rice selector and chooses the smallest payload. It prefers the
legacy frame-global choice on a size tie, then lower squared reconstruction
error, then lower mode number.

The mode consumes the existing directory byte. No per-payload control bits or
additional directory bytes are charged.

## Entropy, quantization, and limits

Eight-bit streams retain Rice parameters 0 through 8 and folded residuals at
most 510. High-bit streams retain parameters 0 through 16 and the
depth-dependent folded limits from version one. Zero-run syntax, canonical
varints, Rice bit order/padding, quantization steps, checked dimensions, and
payload-contiguity rules are unchanged.

High-bit entropy mode 18 encodes the tile's zigzag-folded residuals in raster
order as consecutive blocks of at most 128 symbols. Each block begins on a
byte boundary with an unsigned one-byte width `w`, followed by exactly
`ceil(n × w / 8)` payload bytes for the block's `n` symbols. Symbols are
unsigned `w`-bit integers in least-significant-bit-first order. Width zero
represents an all-zero block and has no payload bytes. The final partial byte
must have zero padding.

The decoder rejects a width greater than the bit width of twice the maximum
sample value, a decoded residual outside that same folded limit, truncation,
nonzero padding, or trailing payload bytes. Legacy version-1 streams do not
permit mode 18. The normative format does not prescribe the encoder's mode
selector; the reference speed encoder samples a source row, considers mode
18 only against fixed-parameter Rice, and otherwise retains the established
zero-run/Rice behavior.

Quality 100 has step one. Every prediction mode therefore reconstructs the
source exactly.

## Backward compatibility

The version-2 decoder accepts:

- version-zero 8-bit streams with prediction modes 0 and 1; and
- version-one 10/12/16-bit streams with prediction modes 0 and 1.

Legacy versions containing modes 2 through 4 are malformed. Version 2 rejects
prediction modes greater than 4.
