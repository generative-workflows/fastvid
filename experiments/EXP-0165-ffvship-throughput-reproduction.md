# EXP-0165 — FFVShip published-throughput reproduction

Status: **ACCEPTED**

Date: 2026-07-29

## Question

FFVShip publishes 1,339-frame 1080p results of 7.146 seconds for
SSIMULACRA2 and 38.710 seconds for Butteraugli on a Ryzen 7940HS and RTX
4050 Mobile. Determine why canonical evaluation on an L40S is far slower and
whether Linux, FFVShip version, decode format, metric concurrency, sequence
length, or worker settings explain the discrepancy.

## Controls

Generate deterministic 120-frame 1920x1080 pairs in tmpfs:

- ordinary 8-bit YUV420 H.264;
- native-depth 10-bit YUV444 FFV1 matching canonical metric interchange.

Compare CUDA FFVShip 5.0.0-a and 5.1.1-a with metrics launched separately.
Then vary GPU streams and decoder processes on 5.0.0-a. Finally stream-copy
repeat the ordinary H.264 and 10-bit FFV1 clips to exactly 1,339 frames. Every
reported JSON contained the expected frame count. This is a diagnostic
microbenchmark, not a codec acceptance result.

Host: dual AMD EPYC 9354 (64 physical cores total), NVIDIA L40S, Linux.

## Version and format isolation (120 frames)

Initial settings were three GPU streams and two decoder processes.

| Version | Input | SSIM wall | Butter wall | Approx. peak RSS |
|---|---|---:|---:|---:|
| 5.0.0 | YUV420p8 H.264 lossless | 5.84 s | 5.87 s | 3.6 GiB |
| 5.1.1 | YUV420p8 H.264 lossless | 6.07 s | 5.96 s | 3.6 GiB |
| 5.0.0 | YUV444p10 FFV1 | 17.28 s | 16.76 s | 12.1 GiB |
| 5.1.1 | YUV444p10 FFV1 | 17.00 s | 16.94 s | 12.1 GiB |

The releases have equivalent throughput. Native-depth YUV444 FFV1 is about
2.9 times slower and uses about 3.4 times the memory of YUV420 H.264. Running
the two metrics concurrently took 7.01 seconds for YUV420 and 26.84 seconds for
YUV444p10: concurrency still beats sequential wall time, but each native-depth
process slows substantially while sharing the host/GPU.

## Worker sweep

On the 120-frame YUV420 Butteraugli control, varying GPU streams with two
decoder processes produced 5.82--6.34 seconds. Decoder-process count dominated:

| GPU streams | Decoder processes | Wall | Peak RSS |
|---:|---:|---:|---:|
| 4 | 1 | 3.80 s | 1.90 GiB |
| 4 | 2 | 6.13 s | 3.63 GiB |
| 4 | 3 | 6.56 s | 5.35 GiB |
| 4 | 6 | 10.04 s | 10.2 GiB |

With one decoder process, two GPU streams were best: 3.59 seconds versus
3.81--3.90 seconds for one, three, four, or six streams. On native-depth
YUV444p10, two GPU streams and one decoder process were also best: 9.27 seconds
SSIM and 9.29 seconds Butteraugli, versus about 17 seconds at three/two.

## Exact published-length reproduction

A normally compressed H.264 pair (`libx264`, CRF 20) took 2.73 seconds SSIM and
3.31 seconds Butteraugli for 120 frames at four/one. Stream-copy repetition to
exactly 1,339 frames gave:

| Input | SSIM wall | SSIM fps | Butter wall | Butter fps |
|---|---:|---:|---:|---:|
| Published Ryzen/4050 result | 7.146 s | 187.4 | 38.710 s | 34.6 |
| Linux L40S, H.264 YUV420p8 | 6.02 s | 222.4 | 8.06 s | 166.1 |
| Linux L40S, FFV1 YUV444p10 | 36.19 s | 37.0 | 36.45 s | 36.7 |

The published result is reproducible—and exceeded—on Linux when the sequence
is long and cheaply decoded. Linux is not the primary problem. Long sequences
amortize FFMS2 indexing, allocation, and pipeline startup dramatically.
Native-depth FFV1/YUV444 decoding and memory movement erase SSIMULACRA2's cheap
kernel advantage, making SSIM and Butteraugli take nearly the same wall time.

## Canonical validation

Run the unchanged six-group rejection tier with FFVShip 5.0.0-a, two GPU
streams, and one decoder process:

```sh
PYTHONPATH=. python3 scripts/evaluate.py --tier rejection \
  --output /tmp/fastvid-v7-six-group-g2t1-rejection.json \
  --ffvship-revision v5.0.0 \
  --ffvship-build '5.0.0-a CUDA, gpu streams 2, decoder processes 1' \
  --ffvship-gpu-id 0 --ffvship-gpu-threads 2 --ffvship-threads 1 \
  --quality-temp /dev/shm
```

| Settings | Metric time | Conversion time | Evaluator wall | Peak RSS | Result |
|---|---:|---:|---:|---:|---|
| GPU 3, decoder 2 | 113.177 s | 65.277 s | 211.897 s | ~21.4 GiB | pass |
| GPU 2, decoder 1 | 58.993 s | 59.560 s | 148.239 s | ~10.9 GiB | pass |

Minimum SSIMULACRA2 `94.112907409668`, maximum Butteraugli
`0.985730707645416`, and compression ratio `6.188000859134071` were identical.
The optimized settings reduce canonical rejection wall time by 30.0%, metric
time by 47.9%, and peak memory by roughly half.

## Decision

Adopt two FFVShip GPU streams and one decoder process as canonical evaluator
defaults on this machine. Retain explicit report fields so another reference
host can reproduce or deliberately retune them. Continue using concurrent
metrics: despite per-process contention, their combined wall time remains lower
than sequential execution. The remaining dominant limitation is short, 4K,
native-depth lossless input rather than Linux or FFVShip release version.

Artifacts:

- `/tmp/fastvid-v7-six-group-g2t1-rejection.json`;
- `/tmp/ffvship-separate-results.tsv`;
- `/tmp/ffvship-worker-results.tsv`;
- `/tmp/ffvship-one-decoder-results.tsv`;
- `/tmp/ffvship-highdepth-worker-results.tsv`.
