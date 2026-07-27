# CUDA feedback summary

GP/s counts full-resolution luma pixels. Encode numbers are medians per sample, then a geometric mean across samples. CUDA decode is complete-call timing. Minimums and pass counts expose scaling failures hidden by aggregates.

## Speed

| Axis | Q | Setting | Samples | Geo. GP/s | Min GP/s | Target | Passing |
|---|---:|---|---:|---:|---:|---:|---:|
| rust_encode | 90 | 1 threads | 24 | 0.032976 | 0.020551 | >3 | 0/24 |
| rust_encode | 90 | 4 threads | 24 | 0.089014 | 0.056606 | >3 | 0/24 |
| rust_encode | 100 | 1 threads | 24 | 0.027932 | 0.021730 | >3 | 0/24 |
| rust_encode | 100 | 4 threads | 24 | 0.075758 | 0.056287 | >3 | 0/24 |
| cuda_decode | 90 | dram | 24 | 2.931514 | 0.443144 | >5 | 6/24 |
| cuda_decode | 90 | vram | 24 | 3.523121 | 0.441946 | >5 | 6/24 |
| cuda_decode | 100 | dram | 24 | 2.342619 | 0.466651 | >5 | 2/24 |
| cuda_decode | 100 | vram | 24 | 3.297352 | 0.470136 | >5 | 7/24 |
| cuda_encode | 90 | vram | 24 | 2.121994 | 0.293499 | >3 | 7/24 |
| cuda_encode | 100 | vram | 24 | 2.091597 | 0.291763 | >3 | 7/24 |

## Rate and quality

| Q | Samples | Total ratio | >15x | Min Y XPSNR | >50 dB | Exact |
|---:|---:|---:|---:|---:|---:|---:|
| 90 | 24 | 11.687517x | 8/24 | 51.9589 dB | 24/24 | 0/24 |
| 100 | 24 | 5.950056x | 4/24 | inf | 24/24 | 24/24 |

## 1080p q90 slice

15 1920x1080 samples are reported separately because fixed launch and orchestration costs make the 4K-only result non-representative.

| Axis | Setting | Geo. GP/s | Min GP/s | Passing |
|---|---|---:|---:|---:|
| rust_encode_1080p | 1 threads | 0.032874 | 0.027222 | 0/15 |
| rust_encode_1080p | 4 threads | 0.088870 | 0.073617 | 0/15 |
| cuda_decode_1080p | dram | 2.713831 | 1.700617 | 1/15 |
| cuda_decode_1080p | vram | 3.395191 | 2.741834 | 1/15 |
| cuda_encode_1080p | vram | 2.245504 | 1.924737 | 2/15 |

Q90 totals 8.366150x compression; 3/15 samples exceed 15x. Minimum luma XPSNR is 51.9589 dB and 15/15 exceed 50 dB.

The four INSTRUCTIONS targets are conjunctive. An aggregate pass does not override a failing sample. Rust encode is the CPU correctness/reference baseline; every reported CUDA encode stream was checked byte-for-byte against it.
