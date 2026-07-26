# Fastvid experimental high-bit bitstream, version 5

Status: **experimental parallel-format candidate**

Version 5 retains version 2's full-tile clamp-gradient prediction while
bounding entropy state inside each access tile. Existing production encoders
continue to emit version 2; version 4 retains its rejected diagnostic syntax.
Header and directory sizes, tile geometry, plane order, quantization, and
frame-level random access are unchanged.

## Header and directory

The header is the 32-byte version-2 high-bit header with byte 4 set to 5.
Version 5 is valid only for high-bit Gray or planar YUV 4:2:2 layouts.

Every version-5 tile has:

| Field | Value |
|---|---:|
| directory entropy mode | 19 |
| directory prediction mode | 6 |

Other pairings are malformed. Versions 1, 2, and 4 retain their existing mode
sets and validation.

## Full-tile predictor

Prediction mode 6 applies the version-2 clamp-gradient predictor over the
complete access tile in raster order:

```text
prediction = clamp(left + above - upper_left, 0, sample_max)
```

Neighbors outside the tile are zero. The encoder quantizes
`source - prediction`, reconstructs with the signaled step, and uses
reconstructed neighbors. This dependency graph permits an exact anti-diagonal
wavefront schedule with `tile_width + tile_height - 1` rounds; execution order
does not alter the canonical raster residual order.

## Entropy shards

The tile's raster folded residuals are divided into consecutive shards of at
most 4,096 symbols. Shards may cross predictor rows because prediction remains
full-tile. Their symbol counts are implicit from tile geometry.

Each shard is:

```text
u8 shard_mode
little-endian u16 body_length
byte[body_length] body
```

The 16-bit length is sufficient canonically: fixed-block mode represents any
valid 4,096-symbol shard in at most 8,736 bytes, and the encoder selects no
larger body.

Shard mode 0 uses the version-2 canonical zero-run representation. Modes 1–17
use the version-4 four-lane Rice body. Mode 18 uses consecutive version-2
fixed blocks of at most 128 symbols. Other modes are malformed.

For equal body lengths the encoder's canonical preference order is zero-run,
Rice, then fixed block.

## Canonical validation

The decoder derives the exact shard and symbol counts from tile geometry. It
rejects:

- an unsupported version or directory mode pairing;
- a truncated shard header, body, Rice length table, or fixed block;
- a body or Rice lane length exceeding its enclosing payload;
- an invalid fixed-block width;
- a residual above twice the signaled sample maximum;
- a zero run exceeding its shard;
- nonzero Rice or fixed-block padding; and
- bytes left in a mode body, shard, tile, or stream.

Quality 100 retains step one and reconstructs every sample exactly.
Directory offsets remain canonical and contiguous, so individual access
tiles remain directly decodable.

## Parallel contract

At default 256x128 luma geometry, exact full-tile prediction has 383
anti-diagonal rounds. Entropy shards contain at most 4,096 symbols,
fixed blocks at most 128 symbols, and each Rice lane at most 1,024 symbols.
A conforming implementation may predict wavefronts and encode shards or lanes
concurrently, but must emit this canonical byte sequence.

Parallel output does not require an append mutex: workers first classify and
count their shards, an exclusive scan assigns byte ranges, and workers then
write disjoint ranges.
