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
```

The demo generates a deterministic synthetic YUV 4:2:2 frame, measures encode
and decode time, compression ratio, per-plane PSNR, and luma block SSIM, and
writes the encoded frame. Run `cargo run -- --help` for raw-frame commands.

Design decisions live in `specs/`, literature notes in `research/`, and
data-backed development records in `experiments/`.

## Current benchmark

Deterministic 1920x1080 YUV422p8 frame on a 4-vCPU AMD EPYC-Genoa VM,
Rust 1.97.1, release mode. These development measurements are not a substitute
for the representative corpus tracked by EXP-0003.

| Quality | Threads | Encoded | Ratio | Encode | Decode | Y PSNR |
|---:|---:|---:|---:|---:|---:|---:|
| 100 | 1 | 37,080 B | 111.845x | 39.3 MP/s | 82.6 MP/s | exact |
| 100 | 4 | 37,080 B | 111.845x | 121.9 MP/s | 198.0 MP/s | exact |
| 75 | 1 | 652,138 B | 6.359x | 36.3 MP/s | 61.8 MP/s | 43.123 dB |
| 75 | 4 | 652,138 B | 6.359x | 127.4 MP/s | 183.2 MP/s | 43.123 dB |
