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
scripts/benchmark-access-corpus.sh
```

The demo generates a deterministic synthetic YUV 4:2:2 frame, measures encode
and decode time, compression ratio, per-plane PSNR, and luma block SSIM, and
writes the encoded frame. Run `cargo run -- --help` for raw-frame commands.

Design decisions live in `specs/`, literature notes in `research/`, and
data-backed development records in `experiments/`.

## Current benchmark

Corpus v2, 4-vCPU AMD EPYC-Genoa VM, Rust 1.97.1, release mode. These are
single-trial development baselines; release comparisons use the warm-up plus
five-trial protocol in [the evaluation methodology](EVALUATION_METHODOLOGY.md).

| Quality | Threads | Geo. ratio | Mean encode | Raw encode | Mean decode | Raw decode | Mean Y PSNR | Mean SSIM |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 90 | 1 | 7.308x | 27.90 MP/s | 55.81 MB/s | 55.41 MP/s | 110.83 MB/s | 49.868 dB | 0.996557 |
| 100 | 1 | 5.013x | 27.38 MP/s | 54.77 MB/s | 54.01 MP/s | 108.03 MB/s | exact | 1.000000 |

The aggregate ratio is strongly influenced by procedural UI/chroma content;
natural-camera quality-100 outliers range from 1.777x to 1.970x. Results must
therefore remain grouped by content class and resolution. See
[EXP-0008](experiments/EXP-0008-corpus-v2-expansion.md).

On the three-video subset, gated GOP-12 temporal prediction improves the
quality-90 geometric ratio from 5.190x to 6.550x while increasing one-thread
encode/decode throughput from 25.94/59.81 to 47.04/75.33 MP/s. Its aggregate
encoded bitrate falls from 154.67 to 127.56 Mb/s (19.33 to 15.95 MB/s).
At quality 100, GOP-12 reduces aggregate bitrate from 247.66 to 206.73 Mb/s
(30.96 to 25.84 MB/s) while reaching 38.67/62.48 MP/s one-thread
encode/decode throughput. See
[EXP-0005](experiments/EXP-0005-gated-temporal-prediction.md) and
[EXP-0007](experiments/EXP-0007-temporal-write-elision.md).

For isolated quality-90 one-thread frame access across all six corpus-v2
videos, all-intra has 34.65/39.20/39.70 ms median/p95/worst latency. GOP-12
has 88.33/409.69/434.13 ms and averages 5.5 decoded frames per requested
frame. These are five-trial codec-only warm-cache results; see
[EXP-0009](experiments/EXP-0009-single-frame-access.md).
