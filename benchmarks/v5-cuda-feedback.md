# CUDA feedback summary

GP/s counts full-resolution luma pixels. Encode numbers are medians per sample, then a geometric mean across samples. CUDA decode is complete-call timing. Minimums and pass counts expose scaling failures hidden by aggregates.

## Speed

| Axis | Q | Setting | Samples | Geo. GP/s | Min GP/s | Target | Passing |
|---|---:|---|---:|---:|---:|---:|---:|
| rust_encode | 90 | 1 threads | 24 | 0.033390 | 0.021713 | >3 | 0/24 |
| rust_encode | 90 | 4 threads | 24 | 0.086448 | 0.063322 | >3 | 0/24 |
| rust_encode | 100 | 1 threads | 24 | 0.028212 | 0.021649 | >3 | 0/24 |
| rust_encode | 100 | 4 threads | 24 | 0.076361 | 0.050249 | >3 | 0/24 |
| cuda_decode | 90 | dram | 24 | 3.029134 | 0.457206 | >5 | 5/24 |
| cuda_decode | 90 | vram | 24 | 2.535451 | 0.426871 | >5 | 4/24 |
| cuda_decode | 100 | dram | 24 | 2.708725 | 0.519445 | >5 | 4/24 |
| cuda_decode | 100 | vram | 24 | 2.137832 | 0.523017 | >5 | 2/24 |

## Rate and quality

| Q | Samples | Total ratio | >15x | Min Y XPSNR | >50 dB | Exact |
|---:|---:|---:|---:|---:|---:|---:|
| 90 | 24 | 11.687517x | 8/24 | 51.9589 dB | 24/24 | 0/24 |
| 100 | 24 | 5.950056x | 4/24 | inf | 24/24 | 24/24 |

## 1080p q90 slice

Fifteen 1920x1080 samples are reported separately because fixed launch and orchestration costs make the 4K-only result non-representative.

| Axis | Setting | Geo. GP/s | Min GP/s | Passing |
|---|---|---:|---:|---:|
| rust_encode_1080p | 1 threads | 0.033321 | 0.027406 | 0/15 |
| rust_encode_1080p | 4 threads | 0.084454 | 0.071467 | 0/15 |
| cuda_decode_1080p | dram | 2.921790 | 2.069232 | 1/15 |
| cuda_decode_1080p | vram | 2.372671 | 1.411177 | 2/15 |

Q90 totals 8.366150x compression; 3/15 samples exceed 15x. Minimum luma XPSNR is 51.9589 dB and 15/15 exceed 50 dB.

The four INSTRUCTIONS targets are conjunctive. An aggregate pass does not override a failing sample, and the Rust encode rows are a correctness/reference baseline—not a claim about the not-yet-implemented CUDA encoder.
