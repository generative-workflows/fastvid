# Version-5 shard-local order-0 screening

The oracle independently keeps the current zero-run/Rice/fixed-block body or substitutes a fully charged order-0 body in each 4,096-symbol shard. Order-0 bytes include normalized tables, final states, byte rounding, and the existing three-byte shard record header. Stream header and directory bytes are retained.

| Scope | Q | Frames | Winning shards | Current compression | Charged oracle | Complete-byte saving | Min Y XPSNR | >15x and >50 dB |
|---|---:|---:|---:|---:|---:|---:|---:|:---:|
| corpus | 80 | 350 | 765464/856288 | 10.532200x | 11.807076x | 10.798% | 34.4485 dB | no |
| 1080p | 80 | 130 | 101814/132600 | 9.498513x | 10.441503x | 9.031% | 34.4485 dB | no |
| 4k | 80 | 170 | 643267/691050 | 10.410991x | 11.675390x | 10.830% | 39.7115 dB | no |
| corpus | 85 | 350 | 755200/856288 | 9.516107x | 10.470392x | 9.114% | 36.8623 dB | no |
| 1080p | 85 | 130 | 98933/132600 | 8.687031x | 9.338974x | 6.981% | 36.8623 dB | no |
| 4k | 85 | 170 | 635016/691050 | 9.386405x | 10.337576x | 9.201% | 41.7807 dB | no |
| corpus | 90 | 350 | 740836/856288 | 8.225066x | 8.879407x | 7.369% | 40.0209 dB | no |
| 1080p | 90 | 130 | 99188/132600 | 7.557306x | 7.970710x | 5.187% | 40.0209 dB | no |
| 4k | 90 | 170 | 619356/691050 | 8.103217x | 8.757242x | 7.468% | 44.2024 dB | no |
| corpus | 95 | 350 | 665699/856288 | 6.519944x | 6.881465x | 5.254% | 45.3638 dB | no |
| 1080p | 95 | 130 | 94371/132600 | 6.010850x | 6.216949x | 3.315% | 45.3638 dB | no |
| 4k | 95 | 170 | 548429/691050 | 6.418263x | 6.780093x | 5.337% | 49.0757 dB | no |
| corpus | 100 | 350 | 805704/856288 | 3.923606x | 6.332831x | 38.043% | inf | no |
| 1080p | 100 | 130 | 116993/132600 | 3.633484x | 5.689550x | 36.138% | inf | no |
| 4k | 100 | 170 | 665584/691050 | 3.859763x | 6.248131x | 38.225% | inf | no |
| corpus-frame-oracle | adaptive | 350 | 729620/856288 | 7.269969x | 8.127258x | 10.548% | 50.0476 dB | no |
| 1080p-frame-oracle | adaptive | 130 | 102329/132600 | 5.818175x | 6.688786x | 13.016% | 50.0529 dB | no |
| 4k-frame-oracle | adaptive | 170 | 604353/691050 | 7.472441x | 8.198322x | 8.854% | 50.1507 dB | no |

The optimistic per-frame quality control uses q80 for 43, q85 for 27, q90 for 67, q95 for 133, q100 for 80 frames, matching the full-frame quality audit.

This is a screening bound, not a claimed format result. A passing candidate still requires exact payload materialization plus matched Rust/CUDA encode/decode measurements.
