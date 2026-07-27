# Version-5 full-corpus rate-distortion sweep

Weighted compression uses total raw bytes divided by total encoded bytes. The quality gate is the minimum luma XPSNR across samples, so an aggregate average cannot hide a failing input.

| Scope | Q | Step | Samples | Compression | >15x samples | Min Y XPSNR | >50 dB samples | Simultaneous aggregate pass |
|---|---:|---:|---:|---:|---:|---:|---:|:---:|
| corpus | 80 | 17 | 24 | 14.678455x | 9/24 | 46.2430 dB | 18/24 | no |
| 1080p | 80 | 17 | 15 | 10.596637x | 3/15 | 46.2430 dB | 11/15 | no |
| corpus | 85 | 13 | 24 | 13.377732x | 9/24 | 49.2855 dB | 23/24 | no |
| 1080p | 85 | 13 | 15 | 9.629467x | 3/15 | 49.2855 dB | 14/15 | no |
| corpus | 90 | 9 | 24 | 11.687517x | 8/24 | 51.9589 dB | 24/24 | no |
| 1080p | 90 | 9 | 15 | 8.366150x | 3/15 | 51.9589 dB | 15/15 | no |
| corpus | 95 | 5 | 24 | 9.472848x | 6/24 | 56.4609 dB | 24/24 | no |
| 1080p | 95 | 5 | 15 | 6.675852x | 3/15 | 56.4609 dB | 15/15 | no |
| corpus | 100 | 1 | 24 | 5.950056x | 4/24 | inf | 24/24 | no |
| 1080p | 100 | 1 | 15 | 4.003757x | 2/15 | inf | 15/15 | no |
| corpus-oracle | adaptive | mixed | 24 | 14.352886x | 9/24 | 50.0476 dB | 24/24 | no |
| 1080p-oracle | adaptive | mixed | 15 | 10.215203x | 3/15 | 50.0878 dB | 15/15 | no |

The content-adaptive oracle chooses the coarsest tested step that keeps each individual sample above 50 dB. It uses q80 for 18, q85 for 5, q90 for 1 corpus samples; the 1080p selection uses q80 for 11, q85 for 3, q90 for 1.

Q100 is the exactness control. The speed targets are evaluated separately; this table locates the deterministic rate-quality boundary used to choose the next compression experiment.
