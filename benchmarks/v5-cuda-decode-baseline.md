# Version-5 CUDA decode baseline

The first CUDA decoder was measured on an NVIDIA L40 (compute capability 8.9,
46,068 MiB, driver 550.127.08) with CUDA 12.8.93 and PyTorch 2.8.0+cu128.
The input is the first derivative frame at source timestamp 5 seconds from the
CC-BY-SA-4.0 Calotes versicolor corpus-v3 clip, converted deterministically
from YUV422p8 to YUV422p10le and encoded by the Rust v5 oracle at 256x128 tile
geometry.

Twenty recorded trials followed five warmups. Each row measures the complete
Python extension call, including canonical host parsing, metadata transfer,
CUDA entropy decoding, antidiagonal reconstruction, allocation, and the
malformed-stream status synchronization. `dram` also transfers the compressed
stream to the GPU; `vram` currently copies it to the host for parsing.

| Q | Input | Ratio | Median | Decode | Raw bandwidth | Y XPSNR | Y PSNR | SSIM | Max error |
|---:|:---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 90 | DRAM | 11.227x | 1.460 ms | 5.681 GP/s | 22.725 GB/s | 57.986 dB | 51.942 dB | 0.995780 | 4 |
| 90 | VRAM | 11.227x | 1.691 ms | 4.906 GP/s | 19.624 GB/s | 57.986 dB | 51.942 dB | 0.995780 | 4 |
| 100 | DRAM | 5.177x | 1.867 ms | 4.444 GP/s | 17.775 GB/s | exact | exact | 1.0 | 0 |
| 100 | VRAM | 5.177x | 2.725 ms | 3.044 GP/s | 12.174 GB/s | exact | exact | 1.0 | 0 |

The q90 point simultaneously exceeds the initial >2 GP/s decode, >50 dB
XPSNR, and >8x compression goals on this real-world 4K frame. This is one
sample and one GPU, not yet a corpus aggregate or a complete project result.
VRAM input is slower because the prototype copies the complete stream to the
host to validate and locate shard records; device-side preparation is the
next decoder optimization target.

A single CUPTI activity trace of the q90 VRAM call measured 593.666 us in
antidiagonal reconstruction, 405.888 us in shard entropy decode, 104.799 us
in allocation zeroing, and 307.394 us of CUDA memcpy activity. Nsight Compute
hardware counters were unavailable (`ERR_NVGPUCTRPERM`), so no occupancy,
divergence, or cache-counter claim is made.

The CUDA extension binary SHA-256 was
`6fa86664c07a2e3c26dc1b6242b553add34c3a938483f7ccb96e389f1d41551c`.
The Rust oracle binary SHA-256 was
`224782496805cc15ee86290515010804b613ea4375a96a986accd86a7e654a69`.
Raw rows are in `v5-cuda-decode-baseline.tsv`.
