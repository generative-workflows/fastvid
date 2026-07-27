# Fastvid

Fastvid is an experimental intermediate video codec focused on high fidelity,
high throughput, compact frames, and inexpensive frame/tile access. The current
implementation has a Rust CPU reference and an experimental PyTorch C++/CUDA
version-5 decoder.

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

## Current benchmark snapshot

The current CPU-to-CUDA handoff baseline is the native high-bit version-5
all-intra path on the four-sample procedural corpus. It is intentionally small
enough to rerun during GPU development; the broader corpus remains the
confirmation suite.

### CUDA 4K decoder baseline

The first complete-call CUDA result uses a real-world 3840x2160 10-bit frame
on an NVIDIA L40. Q90 reaches 5.681 GP/s from DRAM and 4.906 GP/s from VRAM at
11.227x compression, 57.986 dB luma XPSNR, and 0.995780 block SSIM. Q100 is
exact and reaches 4.444/3.044 GP/s. This is one-sample evidence; the full
corpus, encoder, and public video/tile APIs remain in progress. See the
[CUDA decoder baseline](benchmarks/v5-cuda-decode-baseline.md).

### Full spatial feedback baseline

The broader confirmation run covers the first frame of all 24 corpus-v3
codec samples from 360p through 4K. At q90, all 24 exceed 50 dB luma XPSNR,
but total compression is 11.688x and only 8/24 exceed 15x. CUDA complete-call
decode has a 3.029 GP/s DRAM geometric mean and exceeds 5 GP/s on 5/24 cases.
The 15-sample 1080p slice reaches 2.922 GP/s from DRAM and 2.373 GP/s from
VRAM, exposing fixed overhead hidden by the earlier 4K-only result. The Rust
reference encoder reaches 0.086 GP/s at q90/four threads; CUDA encoding is not
yet implemented. Q100 is exact on all 24 samples. See the
[joint feedback report](benchmarks/v5-cuda-feedback.md).

### Rate and quality

| Quality | Total ratio | Geo. ratio | Bits/luma px | Mean bitrate | Mean Y PSNR | Mean SSIM | Mean Y XPSNR | Worst error |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 60 | 10.273x | 11.328x | 3.654 | 140.32 Mb/s | 40.756 dB | 0.953148 | 38.308 dB | 1024 |
| 75 | 9.285x | 9.564x | 4.139 | 160.30 Mb/s | 44.762 dB | 0.975260 | 42.261 dB | 640 |
| 90 | 7.357x | 7.325x | 4.940 | 187.42 Mb/s | 52.406 dB | 0.994336 | 49.808 dB | 256 |
| 95 | 5.213x | 5.678x | 6.488 | 239.01 Mb/s | 57.965 dB | 0.998358 | 55.277 dB | 128 |
| 100 | 3.572x | 3.605x | 9.237 | 336.64 Mb/s | exact | 1.000000 | exact | 0 |

Bitrate assumes 24 fps. Cross-depth aggregate ratios are useful for screening,
but per-sample rows are authoritative because raw storage cost changes with bit
depth.

### CPU speed and thread scaling

| Quality | Threads | Encode | Encode scaling | Decode | Decode scaling | Raw encode | Raw decode |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 90 | 1 | 0.0503 GP/s | 1.00x | 0.0723 GP/s | 1.00x | 0.201 GB/s | 0.289 GB/s |
| 90 | 2 | 0.0558 GP/s | 1.11x | 0.0725 GP/s | 1.00x | 0.223 GB/s | 0.290 GB/s |
| 90 | 4 | 0.1268 GP/s | 2.52x | 0.1509 GP/s | 2.09x | 0.507 GB/s | 0.603 GB/s |
| 100 | 1 | 0.0407 GP/s | 1.00x | 0.0590 GP/s | 1.00x | 0.163 GB/s | 0.236 GB/s |
| 100 | 2 | 0.0482 GP/s | 1.18x | 0.0646 GP/s | 1.09x | 0.193 GB/s | 0.258 GB/s |
| 100 | 4 | 0.1101 GP/s | 2.70x | 0.1448 GP/s | 2.46x | 0.441 GB/s | 0.579 GB/s |

These are geometric means of per-sample medians from five post-warmup trials on
an AMD EPYC Genoa four-vCPU host. The weak two-thread result is an implementation
issue in the current CPU path, not a format limit. Full per-sample results,
environment details, metric definitions, and the frozen binary identity are in
[the version-5 CPU baseline](benchmarks/v5-cpu-baseline.md).

### Preserved matched OpenAPV target

OpenAPV is a fixed reference and is not rerun in the fast feedback loop. These
preserved rows use the same 10-bit `high-precision-motion10` input and host:

| Codec / slot | Setting | Ratio | Encode | Decode | Mean Y PSNR |
|---|---:|---:|---:|---:|---:|
| Fastvid speed | q90, 1 thread | 4.809x | 0.0938 GP/s | 0.0684 GP/s | 52.002 dB |
| OpenAPV fastest | qp23, 1 thread | 4.464x | 0.0812 GP/s | 0.0635 GP/s | 51.736 dB |
| Fastvid max-compression | q90, 1 thread | 5.308x | 0.0169 GP/s | 0.0602 GP/s | 52.002 dB |

The complete fixed-target sweep and caveats are in
[the OpenAPV frontier summary](benchmarks/openapv-frontier-summary.tsv).
Broader 8-bit corpus results and historical Pareto points live in the
[benchmark index](benchmarks/README.md) and [frontier](FRONTIER.md), rather than
being duplicated here.

## Project documentation

- [Evaluation methodology](EVALUATION_METHODOLOGY.md) defines corpus, quality,
  throughput, bitrate, random-access, and fast/slow feedback protocols.
- [Version-5 CPU baseline](benchmarks/v5-cpu-baseline.md) is the current
  CPU-to-CUDA handoff reference.
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
  interleaved order-0 byte-rANS. Experimental high-bit
  [version 5](specs/format-v5.md) specifies the full-tile
  wavefront/bounded-entropy candidate.
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
