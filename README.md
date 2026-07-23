# Fastvid

Fastvid is an experimental, CPU-oriented intermediate video codec. The current
prototype is a deliberately small vertical slice: independently coded tiles,
YUV 4:2:2 or grayscale 8-bit planes, predictive residuals, scalar
quantization, and variable-length integer entropy coding.

The bitstream is **experimental and unstable**. Version zero exists to measure
ideas, not to promise compatibility.

## Quick start

```sh
cargo test
cargo run --release -- demo 1920 1080 90 4 artifacts/demo.fvid
```

The demo generates a deterministic synthetic YUV 4:2:2 frame, measures encode
and decode time, compression ratio, and luma PSNR, and writes the encoded
frame. Run `cargo run -- --help` for raw-frame commands.

Design decisions live in `specs/`, literature notes in `research/`, and
data-backed development records in `experiments/`.

