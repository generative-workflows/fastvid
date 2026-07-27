# Version-5 shard-local order-0 screening

The oracle independently keeps the current zero-run/Rice/fixed-block body or substitutes a fully charged order-0 body in each 4,096-symbol shard. Order-0 bytes include normalized tables, final states, byte rounding, and the existing three-byte shard record header. Stream header and directory bytes are retained.

| Scope | Q | Samples | Winning shards | Current compression | Charged oracle | Complete-byte saving | Min Y XPSNR | >15x and >50 dB |
|---|---:|---:|---:|---:|---:|---:|---:|:---:|
| corpus | 80 | 24 | 27029/37926 | 14.678455x | 17.911841x | 18.052% | 46.2430 dB | no |
| 1080p | 80 | 15 | 10568/15300 | 10.596637x | 11.785717x | 10.089% | 46.2430 dB | no |
| corpus | 85 | 24 | 27049/37926 | 13.377732x | 15.994952x | 16.363% | 49.2855 dB | no |
| 1080p | 85 | 15 | 10370/15300 | 9.629467x | 10.496178x | 8.257% | 49.2855 dB | no |
| corpus | 90 | 24 | 27598/37926 | 11.687517x | 13.678019x | 14.553% | 51.9589 dB | no |
| 1080p | 90 | 15 | 10365/15300 | 8.366150x | 8.944368x | 6.465% | 51.9589 dB | no |
| corpus-oracle | adaptive | 24 | 27245/37926 | 14.352886x | 17.436014x | 17.683% | 50.0476 dB | yes |
| 1080p-oracle | adaptive | 15 | 10753/15300 | 10.215203x | 11.318518x | 9.748% | 50.0878 dB | no |

The adaptive quality control uses q80 for 18, q85 for 5, q90 for 1 samples, matching the EXP-0147 coarsest-tested-step oracle.

This is a screening bound, not a claimed format result. A passing candidate still requires exact payload materialization plus matched Rust/CUDA encode/decode measurements.
