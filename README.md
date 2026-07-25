# Fastvid

Fastvid is an experimental CPU-oriented intermediate video codec focused on
high fidelity, high throughput, compact frames, and inexpensive frame/tile
access.

The current version supports planar 8/10/12/16-bit YUV 4:2:2 and grayscale,
independently coded tiles, spatial prediction, optional short-GOP temporal
prediction, and per-tile adaptive zero-run, Rice, fixed-block high-bit, or
scalar/four-state 8-bit order-0 rANS entropy coding. High-bit raw interchange
uses tightly packed little-endian `u16` samples. The bitstream is experimental
and does not promise backward compatibility.

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

## Maximum-compression full-corpus benchmark

Current maximum-compression snapshot on a 4-vCPU AMD EPYC-Genoa VM with Rust
1.97.1 in release mode. This is the 18-sample corpus-v2 codec track at GOP 1,
one thread, after warm-up and two balanced recorded trials per cell.

| Quality | Threads | Geo. ratio | Encode | Raw encode | Decode | Raw decode | Mean Y PSNR | Mean SSIM |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 90 | 1 | 9.191x | 16.43 MP/s | 32.86 MB/s | 49.21 MP/s | 98.42 MB/s | 49.908 dB | 0.996653 |
| 100 | 1 | 5.983x | 15.95 MP/s | 31.90 MB/s | 47.01 MP/s | 94.02 MB/s | exact | 1.000000 |

Compression is the geometric mean of per-sample raw/encoded ratios. Throughput
and quality columns are arithmetic means of per-sample results; MP/s counts
full-resolution luma pixels and raw MB/s uses actual planar byte counts.
Encoded sizes and reconstruction metrics are thread-invariant.

This table is a current development snapshot, not the full release protocol.
Performance decisions use warm-up plus repeated, balanced trials as defined in
[the evaluation methodology](EVALUATION_METHODOLOGY.md). Per-sample rows must
be retained because synthetic content compresses much more strongly than
camera/noisy content.

This table is not numerically comparable to the automatic frontier graph.
That graph compares three preserved Fastvid binaries on four pinned
fast-feedback cases with mixed q90/q100, GOP 1/12, and one/four-thread
settings, then geometrically aggregates per-case medians. See
[the frontier summary](benchmarks/README.md) for its exact scope.

### Matched OpenAPV reference

The external-reference panel uses native 10-bit YUV 4:2:2, the same
1280x720x24 source bytes, all-intra coding, 256x128 tiles, and one thread.
OpenAPV controls are selected by measured Y-PSNR distance from practical
Fastvid q90, not by assuming nominal controls are equivalent.

| Codec | Control | Ratio | Encode | Decode | Y PSNR |
|---|---:|---:|---:|---:|---:|
| Fastvid speed | q90 | 4.809x | 65.48 MP/s | 69.18 MP/s | 52.002 dB |
| Fastvid practical | q90 | 5.308x | 16.53 MP/s | 58.92 MP/s | 52.002 dB |
| Fastvid maximum | q90 | 5.308x | 16.70 MP/s | 59.78 MP/s | 52.002 dB |
| OpenAPV medium | QP 22 | 4.408x | 17.63 MP/s | 63.47 MP/s | 51.535 dB |
| OpenAPV fastest | QP 23 | 4.464x | 81.18 MP/s | 63.47 MP/s | 51.736 dB |

The Fastvid speed branch is a distinct high-bit point with sampled scalar
fixed-block coding. At q90 it uses 7.18% less bitrate and measures 0.266 dB
higher Y-PSNR than OpenAPV `fastest`; Fastvid is 19.35% slower to encode and
8.99% faster to decode. At the high-fidelity boundary, Fastvid speed q100 is
exact at 2.744x and 66.04 MP/s encode; OpenAPV `fastest` QP0 has maximum error
2 at 1.965x and 63.20 MP/s. These are distinct quality boundaries, not a
nominal-control match.
The [matched graph](benchmarks/openapv-frontier.svg) and
[exact one/four-thread rows](benchmarks/openapv-frontier-summary.tsv) are a
procedural diagnostic, not a broad natural-HDR claim.

### Maximum-compression native high-bit smoke snapshot

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
  [automatic Pareto graph](benchmarks/frontier.svg) show the current internal
  speed, practical-compression, and maximum-compression tradeoffs. A separate
  [matched OpenAPV graph](benchmarks/openapv-frontier.svg) compares native
  10-bit all-intra performance.
- [Corpus documentation](corpus/README.md) describes reproducible media,
  checksums, capability tracks, and licenses.
- [Format specifications](specs/format-v0.md) define the 8-bit v0 syntax;
  [version 1](specs/format-v1.md) adds native 10/12/16-bit samples, and
  [version 2](specs/format-v2.md) adds tile-local predictor modes and the
  high-bit fixed-block entropy mode. The current 8-bit
  [version 3](specs/format-v3.md) adds tile-local scalar and four-state
  interleaved order-0 byte-rANS.
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
