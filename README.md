# Fastvid

Fastvid is an experimental CPU-oriented intermediate video codec focused on
high fidelity, high throughput, compact frames, and inexpensive frame/tile
access.

The current version supports planar 8/10/12/16-bit YUV 4:2:2 and grayscale,
independently coded tiles, spatial prediction, optional short-GOP temporal
prediction, and per-tile adaptive zero-run or Rice entropy coding. High-bit
raw interchange uses tightly packed little-endian `u16` samples. The bitstream
is experimental and does not promise backward compatibility.

## Quick start

```sh
cargo test
cargo run --release -- demo 1920 1080 90 4 artifacts/demo.fvid
scripts/fetch-corpus.sh
scripts/benchmark-feedback.sh
scripts/benchmark-corpus.sh
scripts/benchmark-access-corpus.sh
```

The demo generates a deterministic frame and reports compression, encode/decode
throughput, PSNR, and luma block SSIM. Run `cargo run -- --help` for raw-frame
commands.

## Current benchmark

Current worktree snapshot on a 4-vCPU AMD EPYC-Genoa VM with Rust 1.97.1 in
release mode. This is the 18-sample corpus-v2 codec track at GOP 1 after one
warm-up and one recorded development trial per cell.

| Quality | Threads | Geo. ratio | Encode | Raw encode | Decode | Raw decode | Mean Y PSNR | Mean SSIM |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 90 | 1 | 7.308x | 49.22 MP/s | 98.45 MB/s | 56.07 MP/s | 112.13 MB/s | 49.868 dB | 0.996557 |
| 90 | 4 | 7.308x | 183.57 MP/s | 367.14 MB/s | 186.65 MP/s | 373.31 MB/s | 49.868 dB | 0.996557 |
| 100 | 1 | 5.013x | 43.79 MP/s | 87.58 MB/s | 52.80 MP/s | 105.60 MB/s | exact | 1.000000 |
| 100 | 4 | 5.013x | 168.06 MP/s | 336.11 MB/s | 181.26 MP/s | 362.51 MB/s | exact | 1.000000 |

Compression is the geometric mean of per-sample raw/encoded ratios. Throughput
and quality columns are arithmetic means of per-sample results; MP/s counts
full-resolution luma pixels and raw MB/s uses actual planar byte counts.
Encoded sizes and reconstruction metrics are thread-invariant.

This table is a current development snapshot, not the full release protocol.
Performance decisions use warm-up plus repeated, balanced trials as defined in
[the evaluation methodology](EVALUATION_METHODOLOGY.md). Per-sample rows must
be retained because synthetic content compresses much more strongly than
camera/noisy content.

### Native high-bit smoke snapshot

Six balanced trials, one thread, GOP 1, using the checksummed native high-bit
supplement:

| Depth/sample | Quality | Ratio | Encode | Decode | Quality |
|---|---:|---:|---:|---:|---:|
| 10-bit HDR gradient | 90 | 4.434x | 37.27 MP/s | 44.60 MP/s | 52.00 dB Y PSNR |
| 10-bit HDR gradient | 100 | 2.397x | 34.66 MP/s | 41.65 MP/s | exact |
| 12-bit precision UI | 90 | 6.583x | 41.65 MP/s | 44.67 MP/s | 52.69 dB Y PSNR |
| 12-bit precision UI | 100 | 2.402x | 35.18 MP/s | 41.52 MP/s | exact |
| 10-bit precision motion | 90 | 4.432x | 38.58 MP/s | 52.08 MP/s | 52.00 dB Y PSNR |
| 10-bit precision motion | 100 | 2.396x | 38.06 MP/s | 47.04 MP/s | exact |
| 16-bit precision motion | 90 | 24.325x | 52.47 MP/s | 62.12 MP/s | 52.93 dB Y PSNR |
| 16-bit precision motion | 100 | 2.339x | 37.54 MP/s | 51.22 MP/s | exact |

High-bit planar 4:2:2 storage uses four raw bytes per luma pixel, so its raw
decimal MB/s is four times the listed MP/s. The procedural supplement is a
precision and performance smoke set, not a calibrated natural-HDR quality
corpus.

## Project documentation

- [Evaluation methodology](EVALUATION_METHODOLOGY.md) defines corpus, quality,
  throughput, bitrate, random-access, and fast/slow feedback protocols.
- [Corpus documentation](corpus/README.md) describes reproducible media,
  checksums, capability tracks, and licenses.
- [Format specifications](specs/format-v0.md) define the 8-bit v0 syntax;
  [version 1](specs/format-v1.md) adds native 10/12/16-bit samples.
- [Research index](research/INDEX.md) records openly usable technical sources.
- [`experiments/`](experiments) contains immutable accepted/rejected
  experimental design records and detailed performance history.

## Current limitations

Fastvid does not yet have 4:4:4, HDR metadata, alpha profiles, or a calibrated
natural/production high-bit corpus. High-bit YUV 4:2:2 is benchmarked
separately from the 8-bit headline table. HDR and alpha assets remain
capability diagnostics rather than being silently converted or discarded.
The current [OpenAPV comparison](research/0015-openapv-matched-comparison.md)
uses one procedural 10-bit motion sequence; broader natural and production
high-bit comparisons are still required.
