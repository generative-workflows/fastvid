# Fastvid

Fastvid is an experimental, CPU-oriented intermediate video codec. The current
prototype is a deliberately small vertical slice: independently coded tiles,
YUV 4:2:2 or grayscale 8-bit planes, predictive residuals, scalar
quantization, and per-tile adaptive zero-run or Rice entropy coding.

The bitstream is **experimental and unstable**. Version zero exists to measure
ideas, not to promise compatibility.

## Quick start

```sh
cargo test
cargo run --release -- demo 1920 1080 90 4 artifacts/demo.fvid
scripts/fetch-corpus.sh
scripts/benchmark-corpus.sh
```

The demo generates a deterministic synthetic YUV 4:2:2 frame, measures encode
and decode time, compression ratio, per-plane PSNR, and luma block SSIM, and
writes the encoded frame. Run `cargo run -- --help` for raw-frame commands.

Design decisions live in `specs/`, literature notes in `research/`, and
data-backed development records in `experiments/`.

## Current benchmark

Corpus v1, 4-vCPU AMD EPYC-Genoa VM, Rust 1.97.1, release mode. These are
single-trial development baselines; release comparisons use the warm-up plus
five-trial protocol in [the evaluation methodology](EVALUATION_METHODOLOGY.md).

| Quality | Threads | Geo. ratio | Mean encode | Raw encode | Mean decode | Raw decode | Mean Y PSNR | Mean SSIM |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 90 | 1 | 5.797x | 26.82 MP/s | 53.64 MB/s | 61.58 MP/s | 123.16 MB/s | 49.787 dB | 0.994824 |
| 90 | 4 | 5.797x | 96.93 MP/s | 193.86 MB/s | 189.83 MP/s | 379.66 MB/s | 49.787 dB | 0.994824 |
| 100 | 1 | 3.641x | 24.41 MP/s | 48.82 MB/s | 57.22 MP/s | 114.44 MB/s | exact | 1.000000 |
| 100 | 4 | 3.641x | 92.27 MP/s | 184.54 MB/s | 177.51 MP/s | 355.02 MB/s | exact | 1.000000 |

On the three-video subset, gated GOP-12 temporal prediction improves the
quality-90 geometric ratio from 5.190x to 6.550x while increasing one-thread
encode/decode throughput from 25.94/59.81 to 40.25/72.43 MP/s. Its aggregate
encoded bitrate falls from 154.67 to 127.56 Mb/s (19.33 to 15.95 MB/s).
At quality 100, GOP-12 reduces aggregate bitrate from 247.66 to 206.73 Mb/s
(30.96 to 25.84 MB/s). See
[EXP-0005](experiments/EXP-0005-gated-temporal-prediction.md).
