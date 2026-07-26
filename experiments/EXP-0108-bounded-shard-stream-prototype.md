# EXP-0108 — Bounded-shard stream prototype

Status: **REJECTED**

## Classification

**Parallel-format exploitation** — scalar normative prototype of EXP-0107.

## Hypothesis

A high-bit version-4 prototype with implicit 64-row predictor bands,
4,096-symbol entropy shards, and four byte-aligned Rice lanes should:

- round-trip exactly at q100 and stay within the existing quantizer error
  bound at q90 for 10/12/16-bit input;
- reject truncation, bad lengths, invalid modes, nonzero Rice padding,
  out-of-range residuals, and trailing bytes;
- keep every predictor unit at or below 16,384 samples and every entropy
  state at or below 4,096 symbols;
- remain within 3% aggregate bytes and 5% per sample of unsplit
  clamp-gradient on the native high-bit q90 corpus; and
- establish measured scalar encode/decode cost without claiming a CPU speed
  promotion.

## Modification

Define high-bit stream version 4 and implement it behind a separate
`encode16_parallel` entry point. Existing `encode16` output remains version 2.
Version 4 uses one tile mode pairing:

- prediction mode 5: independently reconstructed clamp-gradient bands of at
  most 64 rows;
- entropy mode 19: implicit 4,096-symbol raster shards, each with an explicit
  mode and payload length; Rice shards use up to four independently delimited
  byte-aligned lanes.

The ordinary high-bit decoder accepts versions 1, 2, and 4.

## Gate

- all stated correctness, malformed-input, span, and rate conditions pass;
- exact stream behavior is deterministic and checksummed;
- fast feedback records encode/decode MP/s, raw MB/s, encoded bitrate, bytes,
  quality, and single-tile access;
- formatting, strict Clippy, and relevant release tests pass.

## Result

The scalar version-4 stream round-trips at 10/12/16 bits, including odd tile
tails, multiple 64-row bands, multiple 4,096-symbol shards, all lane counts,
and independent tile decode. Q100 is exact on every native sample. Q90 stays
inside the existing bounds (maximum errors 4/16/256 at 10/12/16 bits).
Malformed mode pairings, shard lengths, truncation, zero runs, out-of-range
Rice values, and nonzero Rice padding are rejected.

The one-trial native q90 screen measured:

| Sample | Depth | Baseline bytes | v4 bytes | Delta | v4 encode | v4 decode |
|---|---:|---:|---:|---:|---:|---:|
| HDR gradient | 10 | 1,725,830 | 1,791,930 | +3.8300% | 10.245 MP/s | 52.616 MP/s |
| Precision motion | 10 | 18,396,207 | 19,107,066 | +3.8642% | 10.596 MP/s | 64.214 MP/s |
| Precision UI | 12 | 1,181,186 | 1,211,215 | +2.5423% | 10.292 MP/s | 56.398 MP/s |
| Precision motion | 16 | 4,794,825 | 4,991,447 | +4.1007% | 11.314 MP/s | 127.082 MP/s |
| **Aggregate** | mixed | **26,098,048** | **27,101,658** | **+3.8455%** | — | — |

The aggregate exceeds the 3% gate. The gap from EXP-0107's +1.727% model is
mostly a baseline distinction: EXP-0107 compared against exact
zero-run/Rice clamp-gradient, while the current speed stream also exploits
128-symbol block pack. The concrete stream is the authoritative promotion
comparison.

Geometric throughput relative to the current version-2 speed path is 0.1433x
encode and 1.1900x decode. The exact prototype scans all 17 Rice parameters
and allocates separate lane buffers, so its encoder is diagnostic rather
than optimized. Its decoder result is nevertheless a useful signal that
bounded independent streams can help.

Warm-cache independent tile access on the matched 10-bit frame, 40 iterations:

| Variant | Tile-sample throughput | Delta |
|---|---:|---:|
| Version 2 | 129.808 MP/s | — |
| Version 4 | 147.815 MP/s | +13.87% |

The version-4 HDR q90 control is deterministic across two encodes:
`fe96e25ebea7b585702128889b2194629a02fad756932135ef5cb5567b5d63c0`.

Artifacts:

- `artifacts/exp0108-bounded-shard-ab.tsv`
  (`a95215f1a3da0b98a517ebd09888fa4680452619fc98957e7a0486eadfae79fe`);
- `artifacts/exp0108-bounded-shard-access.tsv`
  (`30684f5bf48cc09935f0bc2eaa9a7c9e4a6c4a24f8d372fbcc46808c3b37ec6d`);
- `artifacts/exp0108-version4-control-a.fvid` (stream hash above).

The three bounded-stream release tests, strict Clippy, formatting, script
validation, and the added Lean band/lane proofs pass. The full release suite
is 60 pass / 5 fail, retaining exactly the five stale speed-tier policy tests
documented since EXP-0099; all new version-4 tests pass.

## Decision

Reject 64-row version 4 as a promoted codec format: its +3.846% aggregate
rate misses the gate and its unoptimized scalar encoder is too slow for an
intermediate codec.

Retain the versioned prototype as executable syntax evidence and a decoder
testbed. Exploit its +19.0% full-decode and +13.9% tile-access signals in the
next branch, but remove the avoidable predictor restart cost: full-tile
wavefront prediction with the same raster entropy shards should preserve
the current predictor rate while retaining bounded entropy state. Optimize
Rice selection/allocation only after that format clears the rate gate.

## References

- [Research 0038](../research/0038-lossless-wavefront-scheduling.md)
- [EXP-0107](EXP-0107-combined-wavefront-entropy-model.md)
