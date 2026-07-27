# Version-5 full-frame rate-quality audit

Every one of the 350 corpus frames is scored independently. The headline quality value is the minimum frame-level luma XPSNR; sequence averages are not used for the gate.

| Scope | Q | Frames | Compression | Min frame Y XPSNR | Min frame/plane XPSNR | >50 dB frames | Exact | >15x and >50 dB |
|---|---:|---:|---:|---:|---:|---:|---:|:---:|
| corpus | 80 | 350 | 10.532200x | 34.4485 dB | 31.5084 dB | 43/350 | 0/350 | no |
| 1080p | 80 | 130 | 9.498513x | 34.4485 dB | 31.5084 dB | 28/130 | 0/130 | no |
| 4k | 80 | 170 | 10.410991x | 39.7115 dB | 39.6462 dB | 13/170 | 0/170 | no |
| corpus | 85 | 350 | 9.516107x | 36.8623 dB | 36.8623 dB | 70/350 | 0/350 | no |
| 1080p | 85 | 130 | 8.687031x | 36.8623 dB | 36.8623 dB | 31/130 | 0/130 | no |
| 4k | 85 | 170 | 9.386405x | 41.7807 dB | 39.8584 dB | 35/170 | 0/170 | no |
| corpus | 90 | 350 | 8.225066x | 40.0209 dB | 40.0209 dB | 137/350 | 0/350 | no |
| 1080p | 90 | 130 | 7.557306x | 40.0209 dB | 40.0209 dB | 32/130 | 0/130 | no |
| 4k | 90 | 170 | 8.103217x | 44.2024 dB | 44.2024 dB | 101/170 | 0/170 | no |
| corpus | 95 | 350 | 6.519944x | 45.3638 dB | 42.3901 dB | 270/350 | 0/350 | no |
| 1080p | 95 | 130 | 6.010850x | 45.3638 dB | 42.3901 dB | 99/130 | 0/130 | no |
| 4k | 95 | 170 | 6.418263x | 49.0757 dB | 48.6662 dB | 144/170 | 0/170 | no |
| corpus | 100 | 350 | 3.923606x | inf | inf | 350/350 | 350/350 | no |
| 1080p | 100 | 130 | 3.633484x | inf | inf | 130/130 | 130/130 | no |
| 4k | 100 | 170 | 3.859763x | inf | inf | 170/170 | 170/170 | no |
| corpus-sample-adaptive | adaptive | 350 | 6.364891x | 50.0878 dB | 48.7899 dB | 350/350 | 144/350 | no |
| 1080p-sample-adaptive | adaptive | 130 | 4.725567x | 50.0878 dB | 48.7899 dB | 130/130 | 72/130 | no |
| 4k-sample-adaptive | adaptive | 170 | 6.651276x | 50.8280 dB | 50.7804 dB | 170/170 | 48/170 | no |
| corpus-frame-oracle | adaptive | 350 | 7.269969x | 50.0476 dB | 47.2585 dB | 350/350 | 80/350 | no |
| 1080p-frame-oracle | adaptive | 130 | 5.818175x | 50.0529 dB | 47.2585 dB | 130/130 | 31/130 | no |
| 4k-frame-oracle | adaptive | 170 | 7.472441x | 50.1507 dB | 50.1366 dB | 170/170 | 26/170 | no |

Worst fixed-quality frames:

- q80: `procedural-scene-cuts` frame 7, 34.4485 dB luma XPSNR.
- q85: `procedural-scene-cuts` frame 3, 36.8623 dB luma XPSNR.
- q90: `procedural-scene-cuts` frame 3, 40.0209 dB luma XPSNR.
- q95: `procedural-scene-cuts` frame 4, 45.3638 dB luma XPSNR.
- q100: `bbb-grass-fur` frame 0, inf dB luma XPSNR.

The per-sample adaptive control chooses one quality for every frame in a sample: q80 for 10 samples, q85 for 3 samples, q90 for 4 samples, q95 for 5 samples, q100 for 6 samples.

The optimistic per-frame oracle chooses q80 for 43 frames, q85 for 27 frames, q90 for 67 frames, q95 for 133 frames, q100 for 80 frames. It is a decision bound, not an implemented rate-control result.

All available first-frame encoded-byte and XPSNR controls match EXP-0147 exactly.
