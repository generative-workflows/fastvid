# Fastvid CUDA extension

This directory contains the C++/CUDA PyTorch reference implementation. It
encodes CUDA uint16 YUV 4:2:2 planes to a canonical Rust v5 byte tensor and
decodes one v5 frame from DRAM or VRAM to CUDA planes.

Build in place:

```sh
python -m pip install -v -e ./cuda --no-build-isolation
```

Run the byte-exact Rust-oracle encode/decode test:

```sh
cargo build --release
pytest -q cuda/tests/test_decode_v5.py
```

Measure complete DRAM- and VRAM-input calls:

```sh
python cuda/benchmarks/benchmark_decode_v5.py frame-q90.fvid frame-q100.fvid
python cuda/benchmarks/benchmark_encode_v5.py \
  frame-yuv422p10le.yuv 3840 2160 10 90 100
```

The v5 header and directory are validated on the host. Entropy shards decode
on CUDA (zero-run, four-lane Rice, or fixed blocks), followed by one
antidiagonal reconstruction block per access tile. The returned tensors stay
on the input CUDA device, or on the current CUDA device for a DRAM input.

Encoding uses the same tile-local antidiagonal predictor, exact Rust
quantization, exact zero-run/Rice/fixed-block size selection, and disjoint
shard writes. Four warps emit the independent Rice lanes; fixed-block shards
use a compact selected-shard list and one warp per 128-symbol block. The
baseline copies compact shard sizes to the host for canonical offset/directory
assembly; a device scan is a measured optimization milestone.
