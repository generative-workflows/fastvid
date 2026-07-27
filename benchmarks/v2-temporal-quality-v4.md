# Version-2 temporal rate-quality feasibility screen

All 350 corpus-v4 frames are encoded with GOP 12 for sequences and GOP 1 for stills. Every frame is independently scored with XPSNR; complete per-frame stream headers and directories are charged.

| Scope | Q | Frames | Keyframes | Compression | Min frame Y XPSNR | Min frame/plane XPSNR | >50 dB frames | Exact | >15x and >50 dB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|:---:|
| corpus | 95 | 350 | 42 | 6.632469x | 45.4098 dB | 42.3901 dB | 270/350 | 0/350 | no |
| 1080p | 95 | 130 | 20 | 6.601839x | 45.4098 dB | 42.3901 dB | 99/130 | 0/130 | no |
| 4k | 95 | 170 | 16 | 6.452605x | 49.0966 dB | 48.6656 dB | 144/170 | 0/170 | no |
| corpus | 100 | 350 | 42 | 3.982847x | inf | inf | 350/350 | 350/350 | no |
| 1080p | 100 | 130 | 20 | 3.994766x | inf | inf | 130/130 | 130/130 | no |
| 4k | 100 | 170 | 16 | 3.875598x | inf | inf | 170/170 | 170/170 | no |
| corpus-sample-adaptive | adaptive | 350 | 42 | 5.890771x | 51.7948 dB | 50.1328 dB | 350/350 | 144/350 | no |
| 1080p-sample-adaptive | adaptive | 130 | 20 | 5.242252x | 51.7948 dB | 51.5421 dB | 130/130 | 72/130 | no |
| 4k-sample-adaptive | adaptive | 170 | 16 | 5.922917x | 52.6240 dB | 52.6240 dB | 170/170 | 48/170 | no |

The sequence-consistent sample-adaptive control uses q95 for 22 samples, q100 for 6 samples.

This is a feasibility control for temporal redundancy, not a proposal to promote the version-2 bitstream or its CPU-oriented entropy structure.
