# Version-5 CPU baseline for CUDA

All rows are all-intra on the checksummed native high-bit corpus. Rate and quality are deterministic one-thread rows; timing is the per-sample median of five post-warm-up trials and then a geometric mean across samples. GP/s counts full-resolution luma pixels.

## Rate and quality

| Q | Total ratio | Geo. ratio | Bits/luma px | Mean bitrate | Mean Y PSNR | Mean block SSIM | Mean Y XPSNR | Worst error | Exact |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|:---:|
| 60 | 10.272691x | 11.328358x | 3.653955 | 140.323004 Mb/s | 40.755741 dB | 0.953148155 | 38.307600 dB | 1024 | false |
| 75 | 9.284735x | 9.563841x | 4.138676 | 160.302006 Mb/s | 44.762440 dB | 0.975260277 | 42.261200 dB | 640 | false |
| 90 | 7.356806x | 7.325424x | 4.940314 | 187.420406 Mb/s | 52.405882 dB | 0.994336057 | 49.807650 dB | 256 | false |
| 95 | 5.212993x | 5.677952x | 6.487672 | 239.007988 Mb/s | 57.964599 dB | 0.998358415 | 55.276700 dB | 128 | false |
| 100 | 3.572230x | 3.605204x | 9.236665 | 336.637338 Mb/s | inf dB | 1.000000000 | inf dB | 0 | true |

## Speed and thread scaling

| Q | Threads | Encode GP/s | Decode GP/s | Encode raw GB/s | Decode raw GB/s | Encode scaling | Decode scaling | Enc. efficiency |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 90 | 1 | 0.050327680 | 0.072270725 | 0.201311 | 0.289083 | 1.000000x | 1.000000x | 100.0% |
| 90 | 2 | 0.055843715 | 0.072457176 | 0.223375 | 0.289829 | 1.109602x | 1.002580x | 55.5% |
| 90 | 4 | 0.126814234 | 0.150863207 | 0.507257 | 0.603453 | 2.519771x | 2.087473x | 63.0% |
| 100 | 1 | 0.040718429 | 0.058981646 | 0.162874 | 0.235927 | 1.000000x | 1.000000x | 100.0% |
| 100 | 2 | 0.048245838 | 0.064564422 | 0.192983 | 0.258258 | 1.184865x | 1.094653x | 59.2% |
| 100 | 4 | 0.110138420 | 0.144826597 | 0.440554 | 0.579306 | 2.704879x | 2.455452x | 67.6% |

## Provenance

- Raw quality: `artifacts/exp0135-v5-cpu-quality.tsv`
- Raw speed: `artifacts/exp0135-v5-cpu-speed.tsv`
- Raw XPSNR: `artifacts/exp0135-v5-cpu-xpsnr.tsv`
- Environment: `artifacts/exp0135-v5-cpu-environment.txt`
- Per-sample normalized tables: `benchmarks/v5-cpu-baseline-quality.tsv` and `benchmarks/v5-cpu-baseline-speed.tsv`

XPSNR was the deepest reproducible metric available on this host. This FFmpeg build lacks libvmaf, and the environment lacks pinned DISTS/ColorVideoVDP dependencies. The four native inputs are procedural, so this is a regression and GPU-handoff baseline rather than a natural-content subjective-quality claim.

OpenAPV remains in the separate matched native-10-bit external panel at `benchmarks/openapv-frontier-summary.tsv`; its preserved rows are not pooled into this four-sample cross-depth aggregate.
