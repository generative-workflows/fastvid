# Version-5 full-frame rate-quality audit

Every one of the 254 corpus-v3 frames is scored independently. The headline quality value is the minimum frame-level luma XPSNR; sequence averages are not used for the gate.

| Scope | Q | Frames | Compression | Min frame Y XPSNR | Min frame/plane XPSNR | >50 dB frames | Exact | >15x and >50 dB |
|---|---:|---:|---:|---:|---:|---:|---:|:---:|
| corpus | 80 | 254 | 14.688184x | 34.4485 dB | 31.5084 dB | 35/254 | 0/254 | no |
| 1080p | 80 | 130 | 9.498513x | 34.4485 dB | 31.5084 dB | 28/130 | 0/130 | no |
| corpus | 85 | 254 | 13.356350x | 36.8623 dB | 36.8623 dB | 62/254 | 0/254 | no |
| 1080p | 85 | 130 | 8.687031x | 36.8623 dB | 36.8623 dB | 31/130 | 0/130 | no |
| corpus | 90 | 254 | 11.660067x | 40.0209 dB | 40.0209 dB | 63/254 | 0/254 | no |
| 1080p | 90 | 130 | 7.557306x | 40.0209 dB | 40.0209 dB | 32/130 | 0/130 | no |
| corpus | 95 | 254 | 9.436829x | 45.3638 dB | 42.3901 dB | 174/254 | 0/254 | no |
| 1080p | 95 | 130 | 6.010850x | 45.3638 dB | 42.3901 dB | 99/130 | 0/130 | no |
| corpus | 100 | 254 | 5.735632x | inf | inf | 254/254 | 254/254 | no |
| 1080p | 100 | 130 | 3.633484x | inf | inf | 130/130 | 130/130 | no |
| corpus-sample-adaptive | adaptive | 254 | 7.052671x | 50.0878 dB | 48.7899 dB | 254/254 | 144/254 | no |
| 1080p-sample-adaptive | adaptive | 130 | 4.725567x | 50.0878 dB | 48.7899 dB | 130/130 | 72/130 | no |
| corpus-frame-oracle | adaptive | 254 | 9.154450x | 50.0476 dB | 47.2585 dB | 254/254 | 80/254 | no |
| 1080p-frame-oracle | adaptive | 130 | 5.818175x | 50.0529 dB | 47.2585 dB | 130/130 | 31/130 | no |

Worst fixed-quality frames:

- q80: `procedural-scene-cuts` frame 7, 34.4485 dB luma XPSNR.
- q85: `procedural-scene-cuts` frame 3, 36.8623 dB luma XPSNR.
- q90: `procedural-scene-cuts` frame 3, 40.0209 dB luma XPSNR.
- q95: `procedural-scene-cuts` frame 4, 45.3638 dB luma XPSNR.
- q100: `bbb-grass-fur` frame 0, inf dB luma XPSNR.

The per-sample adaptive control chooses one quality for every frame in a sample: q80 for 10 samples, q85 for 3 samples, q90 for 1 samples, q95 for 4 samples, q100 for 6 samples.

The optimistic per-frame oracle chooses q80 for 35 frames, q85 for 27 frames, q90 for 1 frames, q95 for 111 frames, q100 for 80 frames. It is a decision bound, not an implemented rate-control result.

All available first-frame encoded-byte and XPSNR controls match EXP-0147 exactly.
