# Fastvid

Fastvid is an experimental CPU-oriented intermediate video codec focused on
high fidelity, high throughput, compact frames, and inexpensive frame/tile
access.

The current version supports planar 8/10/12/16-bit YUV 4:2:2 and grayscale,
independently coded tiles, spatial prediction, optional short-GOP temporal
prediction, and per-tile adaptive zero-run, Rice, or 8-bit order-0 rANS
entropy coding. High-bit raw interchange uses tightly packed little-endian
`u16` samples. The bitstream is experimental and does not promise backward
compatibility.

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

Current maximum-compression snapshot on a 4-vCPU AMD EPYC-Genoa VM with Rust
1.97.1 in release mode. This is the 18-sample corpus-v2 codec track at GOP 1,
one thread, after warm-up and two balanced recorded trials per cell.

| Quality | Threads | Geo. ratio | Encode | Raw encode | Decode | Raw decode | Mean Y PSNR | Mean SSIM |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 90 | 1 | 9.203x | 16.18 MP/s | 32.36 MB/s | 46.94 MP/s | 93.88 MB/s | 49.908 dB | 0.996653 |
| 100 | 1 | 5.990x | 15.24 MP/s | 30.48 MB/s | 42.88 MP/s | 85.76 MB/s | exact | 1.000000 |

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

Four to six balanced trials, one thread, using the checksummed native
high-bit supplement. Stills use GOP 1 and motion uses GOP 12.

| Depth/sample | Quality | Ratio | Encode | Decode | Quality |
|---|---:|---:|---:|---:|---:|
| 10-bit HDR gradient | 90 | 5.309x | 16.69 MP/s | 49.03 MP/s | 52.00 dB Y PSNR |
| 10-bit HDR gradient | 100 | 2.949x | 17.51 MP/s | 47.58 MP/s | exact |
| 12-bit precision UI | 90 | 7.697x | 21.12 MP/s | 67.64 MP/s | 53.60 dB Y PSNR |
| 12-bit precision UI | 100 | 4.604x | 17.51 MP/s | 54.21 MP/s | exact |
| 10-bit precision motion | 90 | 5.763x | 17.24 MP/s | 55.18 MP/s | 52.00 dB Y PSNR |
| 10-bit precision motion | 100 | 3.102x | 18.52 MP/s | 53.02 MP/s | exact |
| 16-bit precision motion | 90 | 20.547x | 85.21 MP/s | 248.29 MP/s | 52.93 dB Y PSNR |
| 16-bit precision motion | 100 | 2.061x | 53.84 MP/s | 71.21 MP/s | exact |

High-bit planar 4:2:2 storage uses four raw bytes per luma pixel, so its raw
decimal MB/s is four times the listed MP/s. The procedural supplement is a
precision and performance smoke set, not a calibrated natural-HDR quality
corpus.

## Project documentation

- [Evaluation methodology](EVALUATION_METHODOLOGY.md) defines corpus, quality,
  throughput, bitrate, random-access, and fast/slow feedback protocols.
- [Codec frontier](FRONTIER.md) and its
  [automatic Pareto graph](benchmarks/frontier.svg) show the current speed,
  practical-compression, and maximum-compression tradeoffs.
- [Corpus documentation](corpus/README.md) describes reproducible media,
  checksums, capability tracks, and licenses.
- [Format specifications](specs/format-v0.md) define the 8-bit v0 syntax;
  [version 1](specs/format-v1.md) adds native 10/12/16-bit samples, and
  [version 2](specs/format-v2.md) adds tile-local predictor modes. The current
  8-bit [version 3](specs/format-v3.md) adds tile-local order-0 byte-rANS.
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
