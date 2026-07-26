# Fastvid experimental high-bit bitstream, version 4

Status: **experimental diagnostic; rejected as a default by EXP-0108**

Version 4 adds bounded predictor and entropy execution units to the
10/12/16-bit format. Existing encoders continue to emit version 2 unless the
parallel-format entry point is selected. Header and directory sizes, tile
geometry, plane order, quantization, and frame-level random access are
unchanged.

## Header and directory

The header is the 32-byte version-2 high-bit header with byte 4 set to 4.
Version 4 is valid only for high-bit Gray or planar YUV 4:2:2 layouts.

Every version-4 tile in this prototype has:

| Field | Value |
|---|---:|
| directory entropy mode | 19 |
| directory prediction mode | 5 |

Other pairings are malformed. Versions 1 and 2 retain their existing mode
sets and validation.

## Predictor bands

Prediction mode 5 divides each access tile into consecutive raster row bands
of at most 64 rows. Band geometry is implicit: every band except the last has
64 rows, and the last consumes the remaining rows.

Each band independently applies:

```text
prediction = clamp(left + above - upper_left, 0, sample_max)
```

Missing left, above, and upper-left samples at each band boundary are zero.
The encoder quantizes `source - prediction`, reconstructs with the signaled
step, and uses reconstructed band-local neighbors. Bands never read
predictor state from another band.

## Entropy shards

The folded residuals of each band remain in raster order and are divided
into consecutive shards of at most 4,096 symbols. Shards never cross a
predictor-band boundary. Their symbol counts are implicit from band
geometry.

Each shard is:

```text
u8 shard_mode
little-endian u32 body_length
byte[body_length] body
```

Shard mode 0 uses the version-2 canonical zero-run representation. Modes
1–17 use Rice parameters 0–16 with the lane body below. Other modes are
malformed.

### Four-lane Rice body

Let `lane_count = min(4, shard_symbol_count)`. Symbol `i` belongs to lane
`i mod lane_count`. The body contains:

```text
repeat lane_count - 1 times:
    little-endian u32 lane_length
byte[] lane_0
byte[] lane_1
...
byte[] lane_(lane_count - 1)
```

The final lane length is inferred from `body_length`, the length table, and
the preceding lane lengths. Each lane is the ordinary least-significant-bit
first Rice stream for its assigned symbols and begins on a byte boundary.
Each lane must consume its exact slice and finish with zero padding.

## Canonical validation

The decoder derives the exact number of bands, shards, lanes, and symbols
from tile geometry. It rejects:

- an unsupported version/mode pairing;
- a truncated header, length table, or body;
- a body or lane length exceeding its enclosing payload;
- a Rice or zero-run residual above twice the signaled sample maximum;
- a zero run exceeding its shard;
- nonzero Rice padding or bytes left in a lane;
- bytes left in a zero-run body, shard, tile, or stream.

Quality 100 retains step one and therefore reconstructs every sample exactly.
Directory offsets remain canonical and contiguous, so individual access
tiles remain directly decodable.

## Parallel contract

At default 256x128 luma geometry, predictor bands contain at most 16,384
samples. Zero-run states contain at most 4,096 symbols and each Rice lane at
most 1,024 symbols. A conforming implementation may process bands, shards,
and lanes concurrently wherever their predictor dependencies permit, but
must produce this canonical byte sequence.
