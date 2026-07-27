# Fastvid CUDA extension

This directory contains the C++/CUDA PyTorch reference implementation. The
first milestone decodes one experimental Rust v5 high-bit frame from a uint8
tensor in DRAM or VRAM and returns CUDA uint16 plane tensors.

Build in place:

```sh
python -m pip install -v -e ./cuda --no-build-isolation
```

Run the byte-exact Rust-oracle test:

```sh
cargo build --release
pytest -q cuda/tests/test_decode_v5.py
```

Measure complete DRAM- and VRAM-input calls:

```sh
python cuda/benchmarks/benchmark_decode_v5.py frame-q90.fvid frame-q100.fvid
```

The v5 header and directory are validated on the host. Entropy shards decode
on CUDA (zero-run, four-lane Rice, or fixed blocks), followed by one
antidiagonal reconstruction block per access tile. The returned tensors stay
on the input CUDA device, or on the current CUDA device for a DRAM input.
