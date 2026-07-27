# EXP-0135 — CPU-to-GPU baseline

Status: **ACCEPTED**

## Classification

**Evaluation / goal calibration** — freeze a principled version-5 CPU
reference before moving implementation work to a CUDA node.

## Hypothesis

The future GPU objective can be expressed as constrained optimization only
if one durable matrix records:

- one-, two-, and four-thread encode/decode throughput in GP/s;
- scaling and parallel efficiency relative to one thread;
- raw storage throughput;
- encoded bytes, ratio, bits per luma pixel, stream MB/s, and playback Mb/s;
- PSNR, exact full block-SSIM, maximum error, and q100 equality; and
- a deeper perceptually weighted metric at every rate point.

Separating deterministic rate/quality measurement from repeated timing
should retain principled evidence while avoiding five redundant metric
calculations per cell.

## Protocol

Target the experimental high-bit version-5 all-intra path from source commit
`434561b` / code commit `77a45fa` on the checksummed native high-bit corpus.

Rate/quality:

- qualities 60, 75, 90, 95, and 100;
- one deterministic codec row per sample/quality at one thread;
- encoded bytes include header, directory, shard controls, padding, and
  payload;
- built-in aggregate native-depth PSNR, full luma block-SSIM, and maximum
  error;
- FFmpeg XPSNR on complete decoded sequences, outside timed regions.

Speed/scaling:

- q90 and q100;
- one, two, and four threads;
- one unrecorded warm-up per sample/quality/thread cell;
- five recorded trials;
- per-sample medians, geometric throughput aggregates, thread speedup, and
  parallel efficiency;
- serial execution of every benchmark process.

Record host, compiler, FFmpeg configuration, source/binary hashes, exact
corpus hashes, commands, and raw TSVs. High-bit samples remain separate by
depth in raw data; cross-depth aggregates are labeled screening summaries.

## Gate

This experiment establishes evidence rather than promoting a codec:

- every q100 row must be byte-exact with infinite PSNR and XPSNR;
- deterministic bytes and core quality must agree between quality and speed
  rows at overlapping settings;
- every raw result must retain its per-sample identity and bit depth;
- XPSNR must consume matching native pixel formats without conversion;
- summary generation must derive GP/s, scaling, ratio, bpp, and bitrate from
  named columns;
- benchmark scripts, formatting, strict lint, and existing tests must pass.

## Result

The frozen version-5 binary has SHA-256
`1c493be6131e8752ee55e9c32949e7c2ef9c6a9d6a4a4505d4bd223e900fc072`.
It was measured on four dedicated AMD EPYC Genoa virtual cores with 7.6 GiB
RAM, Linux 7.0.0, Rust 1.97.1, and FFmpeg 8.0.1. The complete environment and
CPU flags are retained in the raw artifact.

Aggregate deterministic rate/quality results:

| Q | Total ratio | Geo. ratio | Mean bits/luma px | Mean Y PSNR | Mean block SSIM | Mean Y XPSNR | Worst error | Exact |
|---:|---:|---:|---:|---:|---:|---:|---:|:---:|
| 60 | 10.272691x | 11.328358x | 3.653955 | 40.755741 dB | 0.953148155 | 38.307600 dB | 1024 | no |
| 75 | 9.284735x | 9.563841x | 4.138676 | 44.762440 dB | 0.975260277 | 42.261200 dB | 640 | no |
| 90 | 7.356806x | 7.325424x | 4.940314 | 52.405882 dB | 0.994336057 | 49.807650 dB | 256 | no |
| 95 | 5.212993x | 5.677952x | 6.487672 | 57.964599 dB | 0.998358415 | 55.276700 dB | 128 | no |
| 100 | 3.572230x | 3.605204x | 9.236665 | exact | 1.000000000 | exact | 0 | yes |

The large absolute worst errors at lower qualities occur on 16-bit data; PSNR
and XPSNR use the native signaled peak. Per-sample rows retain bit depth and
must be used for any fixed maximum-error constraint.

Five-trial post-warm-up speed aggregates:

| Q | Threads | Encode GP/s | Decode GP/s | Encode scaling | Decode scaling | Encode efficiency |
|---:|---:|---:|---:|---:|---:|---:|
| 90 | 1 | 0.050327680 | 0.072270725 | 1.000x | 1.000x | 100.0% |
| 90 | 2 | 0.055843715 | 0.072457176 | 1.110x | 1.003x | 55.5% |
| 90 | 4 | 0.126814234 | 0.150863207 | 2.520x | 2.087x | 63.0% |
| 100 | 1 | 0.040718429 | 0.058981646 | 1.000x | 1.000x | 100.0% |
| 100 | 2 | 0.048245838 | 0.064564422 | 1.185x | 1.095x | 59.2% |
| 100 | 4 | 0.110138420 | 0.144826597 | 2.705x | 2.455x | 67.6% |

Two-thread scaling is consistently weak because multithreaded v5 does not
use the accepted one-thread paired predictor schedule, while two workers do
not provide enough parallel work to amortize its loss and thread
coordination. Four cores overcome more of that cost but remain at 63–68%
encode efficiency. This is a CPU implementation result, not a format limit or
a CUDA scaling prediction.

Every overlapping speed row agrees exactly with the deterministic encoded
bytes and quality fields. All q100 codec and XPSNR rows are exact. XPSNR
consumed native `yuv422p10le`, `yuv422p12le`, and `yuv422p16le` without a
format filter.

Artifacts:

- `artifacts/exp0135-v5-cpu-quality.tsv`
  (`d866794beee17171d8de35fdcf4d0a2e525821841210ac40036f3ad8e7c90c86`);
- `artifacts/exp0135-v5-cpu-speed.tsv`
  (`541d1874e992e9a8142e78a70baa0158329101d5abed0f51ecdf93ce6ab826bf`);
- `artifacts/exp0135-v5-cpu-xpsnr.tsv`
  (`05305c765f3c9d4d957af70e679cefeb8bc104d363a874eacb6c4087c534aff7`);
- `artifacts/exp0135-v5-cpu-environment.txt`
  (`860444af60926c3d2ac7fadae309e6d6f3b0183386d596d7d1ed08f0e22b0ce2`);
- `benchmarks/v5-cpu-baseline-quality-summary.tsv`
  (`2c7b7bb06fea05757b018848ce3a30f499159d631f9e9456f7ac1a1bf133e340`);
- `benchmarks/v5-cpu-baseline-speed-summary.tsv`
  (`e097225cedf9f20d7d693a69502652a9acfb6556d61604823163af3f4fda64fa`).

The complete human-readable report is
[`benchmarks/v5-cpu-baseline.md`](../benchmarks/v5-cpu-baseline.md).

The XPSNR harness streams decoded frames directly into FFmpeg, so metric
evaluation needs only one frame of temporary storage even for a sequence.
All 69 release library tests and nine binary targets pass. Release
all-feature strict Clippy, Rust formatting, every retained shell syntax
check, Python byte-compilation, summary cross-validation, and diff checks
pass.

## Decision

Accept this as the version-5 CPU reference for CUDA work. It is a baseline,
not a new frontier promotion and not a subjective HDR-quality claim: all four
native inputs are deterministic/procedural, and VMAF, DISTS, and
ColorVideoVDP were unavailable on this host.

Express the GPU optimization goal as:

1. predeclare per-sample and aggregate PSNR, block-SSIM, XPSNR, and maximum
   error floors;
2. predeclare end-to-end encode and decode GP/s floors on a named GPU;
3. require byte-identical q100 and scalar-oracle conformance;
4. among candidates satisfying every constraint, minimize complete encoded
   bytes.

Keep the raw per-depth rows authoritative when choosing numeric floors. Add
natural native 10/12-bit and calibrated HDR/video content before treating the
quality constraints as a release target.

## References

- [Research 0018](../research/0018-modern-perceptual-metrics.md)
- [Research 0043](../research/0043-xpsnr-quality-metric.md)
- [EXP-0134](EXP-0134-cuda-handoff-contract.md)
