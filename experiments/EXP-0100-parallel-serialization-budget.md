# EXP-0100 — Parallel serialization budget

Status: **ACCEPTED**

## Classification

**Format exploration** — analytical lower bound for execution shards inside
the existing access tiles.

## Hypothesis

Separating access-tile geometry from 4,096-symbol entropy execution shards can
reduce the maximum entropy-state span from 32,768 to 4,096 symbols and expose
at least 1,000 shards in a 1920x1080 YUV 4:2:2 frame. If each extra shard
boundary is pessimistically charged one `u32` length and one byte of padding,
the structural overhead will remain below 0.025 bits per luma pixel on every
standard 1080p core/high-bit sample.

This is an optimistic lower bound: it does not claim predictor independence,
model reset costs, or an implementation speedup.

## Modification

Add `scripts/audit-parallel-serialization.py`. For each core and native
high-bit sample at default 256x128 access geometry, enumerate exact plane/tile
sample counts and hypothetical 256/512/1024/2048/4096-symbol shards.

The model charges:

- one length for every shard except the last in an access tile (the existing
  tile length delimits the last);
- both two-byte and safe four-byte length alternatives; and
- a pessimistic extra byte-alignment byte per added boundary.

It does not change the codec or bitstream.

## Gate

- reproduce the current 90/216/765 tile counts at 720p/1080p/4K;
- 4,096-symbol rows have maximum span at most 4,096 and at least 1,000 shards
  for every 1080p YUV 4:2:2 sample;
- `u32` lengths plus worst padding remain below 0.025 bpp on those rows; and
- output is deterministic and checksummed.

## Result

The model reproduces all three geometry controls and is byte-deterministic
across repeated runs:

| Resolution | Access tiles | 4,096-symbol shards | Maximum span | `u32` + padding bpp |
|---:|---:|---:|---:|---:|
| 640x360 | 27 | 118 | 4,096 | 0.015799 |
| 1280x720 | 90 | 455 | 4,096 | 0.015842 |
| 1920x1080 | 216 | 1,020 | 4,096 | 0.015509 |
| 3840x2160 | 765 | 4,065 | 4,096 | 0.015914 |

Every one of the 17 core/native-high-bit 1080p rows has exactly 1,020
execution shards and 0.015509 bpp pessimistic structural overhead. The largest
4,096-symbol overhead anywhere in the current compatible corpus dimensions is
0.015914 bpp, below the 0.025-bpp gate.

For the representative 1080p row, the modeled ladder is:

| Shard symbols | Shards | Shards/luma MP | `u32` + padding bpp |
|---:|---:|---:|---:|
| 256 | 16,200 | 7,812.50 | 0.308333 |
| 512 | 8,100 | 3,906.25 | 0.152083 |
| 1,024 | 4,051 | 1,953.61 | 0.073978 |
| 2,048 | 2,033 | 980.42 | 0.035050 |
| 4,096 | 1,020 | 491.90 | 0.015509 |

The artifact is
`artifacts/exp0100-parallel-serialization.tsv`
(`ae62396115bf1bb18608c55d5452cb6fdea7a598f31859f3af5126780072491e`).
Python bytecode compilation passes.

## Decision

Accept the 4,096-symbol point as a viable format-model budget and use it as
the next entropy-lane exploration boundary. This does **not** accept a format
change: the model omits predictor restart cost, actual length distributions,
mode divergence, and runtime. In particular, entropy shards alone do not
parallelize clamp-gradient reconstruction. A subsequent complete-byte model
must compare explicit multi-lane Rice, block pack, and the current payload
before any implementation or syntax change.

## References

- [Research 0037](../research/0037-parallel-hardware-friendly-codecs.md)
- [Research 0028](../research/0028-tile-geometry-tradeoffs.md)
