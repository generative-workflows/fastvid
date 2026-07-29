# Fastvid CUDA extension

This directory contains the single C++/CUDA PyTorch implementation. Live source,
binding, test, and benchmark names are intentionally unversioned. The encoded
format carries bitstream version 1 in its header; implementation experiments do
not create parallel codec versions.

Build in place:

```sh
python -m pip install -v -e ./cuda --no-build-isolation
```

Run the CUDA codec tests:

```sh
pytest -q cuda/tests/test_codec.py
```

Measure complete DRAM- and VRAM-input calls:

```sh
python cuda/benchmarks/benchmark_decode.py frame-q90.fvid frame-q100.fvid
python cuda/benchmarks/benchmark_encode.py \
  frame-yuv422p10le.yuv 3840 2160 10 90 100
```

The header and directory are validated on the host. Entropy shards decode on
CUDA, followed by one antidiagonal reconstruction block per access tile. The
returned tensors stay on the input CUDA device, or on the current CUDA device
for a DRAM input.

Encoding uses the tile-local antidiagonal predictor, format-aware quantization,
zero-run/Rice/fixed-block/order-0 selection, and disjoint shard writes. Four
warps emit independent Rice lanes; fixed-block shards use a compact selected-
shard list and one warp per 128-symbol block. Compact shard sizes are copied to
the host for canonical offset and directory assembly.
