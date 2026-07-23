# Fastvid

Fastvid is an experimental CPU-oriented intermediate video codec focused on
high fidelity, high throughput, compact frames, and inexpensive frame/tile
access.

The current version supports planar 8-bit YUV 4:2:2 and grayscale, independently
coded tiles, spatial prediction, optional short-GOP temporal prediction, and
per-tile adaptive zero-run or Rice entropy coding. The bitstream is experimental
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

## Current benchmark

Current worktree snapshot on a 4-vCPU AMD EPYC-Genoa VM with Rust 1.97.1 in
release mode. This is the 18-sample corpus-v2 codec track at GOP 1 after one
warm-up and one recorded development trial per cell.

| Quality | Threads | Geo. ratio | Encode | Raw encode | Decode | Raw decode | Mean Y PSNR | Mean SSIM |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 90 | 1 | 7.308x | 38.43 MP/s | 76.87 MB/s | 56.13 MP/s | 112.25 MB/s | 49.868 dB | 0.996557 |
| 90 | 4 | 7.308x | 141.52 MP/s | 283.04 MB/s | 177.67 MP/s | 355.34 MB/s | 49.868 dB | 0.996557 |
| 100 | 1 | 5.013x | 32.27 MP/s | 64.54 MB/s | 50.72 MP/s | 101.45 MB/s | exact | 1.000000 |
| 100 | 4 | 5.013x | 131.62 MP/s | 263.24 MB/s | 179.44 MP/s | 358.89 MB/s | exact | 1.000000 |

Compression is the geometric mean of per-sample raw/encoded ratios. Throughput
and quality columns are arithmetic means of per-sample results; MP/s counts
full-resolution luma pixels and raw MB/s uses actual planar byte counts.
Encoded sizes and reconstruction metrics are thread-invariant.

This table is a current development snapshot, not the full release protocol.
Performance decisions use warm-up plus repeated, balanced trials as defined in
[the evaluation methodology](EVALUATION_METHODOLOGY.md). Per-sample rows must
be retained because synthetic content compresses much more strongly than
camera/noisy content.

## Project documentation

- [Evaluation methodology](EVALUATION_METHODOLOGY.md) defines corpus, quality,
  throughput, bitrate, random-access, and fast/slow feedback protocols.
- [Corpus documentation](corpus/README.md) describes reproducible media,
  checksums, capability tracks, and licenses.
- [Format specification](specs/format-v0.md) defines the current bitstream.
- [Research index](research/INDEX.md) records openly usable technical sources.
- [`experiments/`](experiments) contains immutable accepted/rejected
  experimental design records and detailed performance history.

## Current limitations

Fastvid does not yet have native 10/12-bit, 4:4:4, HDR metadata, or alpha
profiles. HDR and alpha assets remain capability diagnostics rather than being
silently converted into headline scores. A direct comparison with
[OpenAPV](https://github.com/AcademySoftwareFoundation/openapv) awaits the
native 10-bit path so throughput, bitrate, and reconstruction quality can be
matched fairly.
