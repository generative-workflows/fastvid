# EXP-0110 — Full-tile bounded entropy shards

Status: **ACCEPTED**

## Classification

**Parallel-format exploitation** — retain EXP-0108's bounded entropy state
while removing its two measured rate disadvantages.

## Hypothesis

A separate high-bit diagnostic format using full-tile clamp-gradient
prediction and independently decodable 4,096-symbol entropy shards should:

- preserve q100 exactly and the existing q90 quantizer error bounds at
  10/12/16 bits;
- keep entropy state bounded to 4,096 symbols, with at most 1,024 symbols per
  Rice lane;
- avoid the 64-row predictor restart loss measured by EXP-0104/0108;
- retain the promoted encoder's 128-symbol fixed-block packing inside each
  shard;
- stay within 1% aggregate bytes and 2% per sample of the version-2 speed
  path on the native high-bit q90 corpus; and
- preserve or improve version 2 scalar decode and independent-tile access
  throughput, without using scalar encode throughput as a promotion gate.

## Modification

Add a version-5 experimental entry point. Each tile uses ordinary full-tile
clamp-gradient prediction. Raster residuals are split into implicit
4,096-symbol shards. Every shard independently selects canonical zero-run,
four-lane Rice, or the existing 128-symbol fixed-block representation and
stores an explicit mode and body length.

Versions 1, 2, and the rejected version 4 retain their exact syntax and
behavior.

## Gate

- all correctness, malformed-input, determinism, and entropy-span conditions
  pass;
- q90 native high-bit aggregate byte delta is at most 1%, with no sample
  above 2%;
- record MP/s, raw MB/s, encoded MB/s and Mbit/s, quality, maximum error, and
  independent-tile access;
- complete release tests, strict Clippy, formatting, and diff checks pass.

## Result

The version-5 stream passes the correctness and rate gates. It retains
full-tile clamp-gradient reconstruction, so its q90 metrics and maximum error
match version 2 exactly; q100 is exact on every 10/12/16-bit native sample.
The format bounds entropy shards to 4,096 symbols, Rice lanes to at most
1,024 symbols, and fixed blocks to 128 symbols. At default 256x128 geometry
the exact predictor dependency graph has 383 anti-diagonal rounds.

The initial 32-bit shard length narrowly missed the per-sample rate gate
(+2.099% on 16-bit motion). Fixed block provides an 8,736-byte upper bound
for every selected 4,096-symbol shard body, so version 5 canonically uses a
16-bit body length. The Lean model proves this bound fits `u16`.

The one-trial native q90 fast-feedback screen measured:

| Sample | Depth | Byte delta | v5 encode | v5 decode | Encoded bitrate |
|---|---:|---:|---:|---:|---:|
| HDR gradient | 10 | +0.5820% | 11.907 MP/s | 61.252 MP/s | 333.288 Mb/s |
| Precision motion | 10 | +0.5799% | 11.901 MP/s | 75.483 MP/s | 148.023 Mb/s |
| Precision UI | 12 | +1.1438% | 12.084 MP/s | 55.911 MP/s | 229.382 Mb/s |
| Precision motion | 16 | +1.6431% | 13.005 MP/s | 122.433 MP/s | 38.989 Mb/s |
| **Aggregate** | mixed | **+0.8009%** | — | — | — |

Aggregate version-2/v5 bytes are 26,098,048/26,307,070. Geometric throughput
relative to version 2 is 0.1742x encode and 1.2607x decode. The encoder result
is expected: the scalar diagnostic constructs zero-run, four-lane Rice, and
fixed-block bodies before selecting one. It is not a speed promotion.

Warm-cache independent access over all 90 tiles of the matched 10-bit frame,
40 iterations:

| Variant | Tile-sample throughput | Delta |
|---|---:|---:|
| Version 2 | 134.624 MP/s | — |
| Version 5 | 179.242 MP/s | +33.14% |

The HDR q90 control is deterministic across two encodes:
`9a3cf708ecdc73f9f8c15a545b41f761ad1ed844c2b8cb4db42118ce587fce37`.

The default-geometry 1920x1080 layout audit measures 216 access tiles and
1,020 entropy shards (491.898 per luma MP), clearing the methodology's
1,000-shard exploration target. Shards are distributed as 255 zero-run, 510
Rice-0, and 255 fixed-block. The maximum predictor DAG span is 383 rounds and
the maximum entropy state is 4,096 symbols; Rice has four directly indexed
lanes with at most 1,024 symbols and fixed block has at most 128.

Shard sample p50/p95/max are all 4,096; encoded-byte p50/p95/max are
1,265/3,123/3,123. Header, directory, shard headers, Rice lane lengths, and
fixed-block controls total 24,224 bytes, 0.093457 bits per luma pixel and
1.395492% of the complete stream. Entropy padding totals 7,182 bits
(0.003464 bits per luma pixel). Full-tile prediction has no restart metadata.

The current encoder's conservative logical scratch bound per active default
luma tile is 162,316 bytes excluding its retained tile payload and allocator
slack: 128 KiB folded residuals, one 512-byte reconstruction row, and the
simultaneously live candidate bodies/lanes. On the one-thread HDR control,
retained tile payloads plus the final copied output peak at 3,468,886 bytes,
0.4182x raw input (1.4182x including the source frame). Final assembly still
copies all 1,730,971 payload bytes through one serial loop. That is an
implementation limitation: the version-5 lengths permit a size scan and
disjoint final writes with zero globally serialized payload bytes.

Artifacts:

- `artifacts/exp0110-full-tile-shards-ab.tsv`
  (`c2cb946d40935b84d08ddbf430d304fa9e5b58da6449addd675ab974d739a5a3`);
- `artifacts/exp0110-full-tile-shards-access.tsv`
  (`1fee7c935b064aabac66558274be482428a110f79ca735ca364254d8f90d4fbe`);
- `artifacts/exp0110-version5-control-a.fvid` (stream hash above).
- `artifacts/exp0110-version5-layout.tsv`
  (`9d9743b0f42d5c72c67d5c6e01fe1f48d13556beb4469455a289455b17f2391d`).

All 66 library tests, the motion/squeeze tests, binary and documentation
tests, strict release Clippy, formatting, diff validation, and the Lean build
pass. Tests cover all depths, q90/q100 bounds, tile decode, entropy analysis,
fixed-block selection, invalid version/mode pairings, unknown shard modes,
excessive short lengths, and the version-4 prohibition on block-pack shards.

## Decision

Accept version 5 as the leading parallel-format candidate, but do not replace
the version-2 speed encoder yet. It clears the rate, quality, decode, access,
determinism, and bounded-state gates while exposing a classify/scan/disjoint
write mapping for CUDA.

The next exploitation step is encoder engineering: avoid constructing three
temporary bodies, calculate exact zero/Rice/block costs in one pass, allocate
only the winning body, and emit into pre-sized shard ranges. Profile that
path before explicit SIMD. Preserve version 2 as the CPU speed frontier and
version 5 as the low-serialization branch until end-to-end encode throughput
is competitive.

## References

- [Research 0038](../research/0038-lossless-wavefront-scheduling.md)
- [EXP-0104](EXP-0104-predictor-band-height-ladder.md)
- [EXP-0108](EXP-0108-bounded-shard-stream-prototype.md)
