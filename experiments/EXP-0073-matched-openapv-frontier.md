# EXP-0073 — Matched OpenAPV frontier reference

Status: **ACCEPTED**

## Classification

**Evaluation exploitation** — extend the three-version Fastvid technology
frontier with a separately scoped external reference panel. This does not add
OpenAPV to Fastvid's internal codec slots or mix incompatible 8-bit and
10-bit aggregates.

## Hypothesis

A fresh same-input 10-bit all-intra measurement will make the speed,
practical-compression, and maximum-compression roles interpretable against
OpenAPV `medium` and `fastest`, while reproducing deterministic bytes and
quality across repeated trials.

## Modification

Add a hash-validating benchmark and graph for:

- all three preserved Fastvid frontier binaries;
- pinned OpenAPV v0.3.0.0 `medium` and `fastest`;
- the checksummed 1280x720, 24-frame, 24-fps
  `high-precision-motion-10` sequence;
- native planar 10-bit YUV 4:2:2, 256x128 tiles, all-intra coding, and one/four
  threads;
- Fastvid q90/q100 and OpenAPV QP 0 plus QP 20 through 24.

For each thread count, use median codec time over five serial trials. Select
the measured OpenAPV QP nearest the practical Fastvid q90 Y-PSNR separately
for each preset. Keep QP0 as an explicitly non-exact high-fidelity boundary,
not a q100 match.

The matched external SVG is separate from the four-case 8-bit Fastvid
frontier graph. `FRONTIER.md` and the README link both views and state their
different scopes.

## Test and gate

- validate every preserved Fastvid binary hash from `frontier.json`;
- record the OpenAPV source archive, encoder, and decoder hashes;
- require trials 1 through 5 for every codec/control/thread cell;
- require stable encoded bytes and reconstruction metrics within each cell;
- use identical input bytes, sample precision, GOP structure, tile geometry,
  and thread count;
- select OpenAPV by measured PSNR distance, never nominal control equality;
- report ratio, bits/pixel, playback bitrate, encode/decode MP/s, PSNR, SSIM,
  maximum error, and quality distance;
- generate deterministic TSV and SVG summaries;
- retain exactly three internal Fastvid frontier slots.

This is a procedural high-bit diagnostic. It cannot support a broad
natural-HDR or production-content superiority claim.

## Result

The complete matrix contains 36 codec/control/thread cells, five trials per
cell, and 180 data rows. Encoded bytes and reconstruction metrics were stable
within every cell. All 16 OpenAPV CTest tests passed before measurement.

At one thread, practical Fastvid q90 measured 5.307903x, 16.552 MP/s encode,
59.467 MP/s decode, 52.002293 dB Y-PSNR, 0.99373118 luma block SSIM, and
133.346224 Mb/s at playback rate. The nearest measured OpenAPV points were:

| Preset | Control | Ratio | Encode | Decode | Bitrate | Y PSNR | Delta |
|---|---:|---:|---:|---:|---:|---:|---:|
| `medium` | QP 22 | 4.408004x | 17.416 MP/s | 62.481 MP/s | 160.568984 Mb/s | 51.534665 dB | -0.467628 dB |
| `fastest` | QP 23 | 4.464067x | 80.724 MP/s | 62.481 MP/s | 158.552448 Mb/s | 51.735588 dB | -0.266705 dB |

Fastvid therefore used 16.95% less bitrate than `medium` and 15.90% less than
`fastest` at slightly higher measured quality. OpenAPV `fastest` encoded
4.88x as quickly at one thread and 4.00x as quickly at four threads. In the
four-thread rows Fastvid decoded 12.96% to 16.66% faster than the two selected
OpenAPV points.

All three Fastvid frontier binaries produced byte-identical q90 and q100
streams on this input. Their one-thread q90 encode range was 16.524--16.864
MP/s and decode range was 58.804--60.045 MP/s, which confirms that the
current slot distinction is confined to the 8-bit implementation.

At the high-fidelity boundary, Fastvid q100 was exact at 2.949766x and
239.947400 Mb/s. OpenAPV QP0 was not exact (`max_error=2`) at approximately
1.966x and 360 Mb/s. The OpenAPV row is consequently a boundary observation,
not a matched q100 point.

The graph generator reproduced byte-identical TSV and SVG outputs on a second
run. The raw matrix has SHA-256
`219bf744c3b4f4ecb8248e5fff0d6ef0bf80114f3a532826e95faa6f908deb44`;
the durable summary has
`fb8e4a60547983d1ddfe9cef89e55d7df69ef9e5612245bf11fc1c0538e158ec`;
and the SVG has
`2b2c936be948de9ef160c1c2692955d8c01b127e389e0e0a4e53ab8bf58c7ac6`.

## Provenance

- input:
  `ff61ed1af3c39e4b12e8a98a8edb94b2d76e2dfcc2f318a62e111b7080b5fbad`;
- OpenAPV v0.3.0.0 source archive:
  `dc5cd1618a07e8b340e12562cae37d612b3a1467ee80d986c477165ae602a37e`;
- OpenAPV encoder:
  `9a65f11bc2d9d0602b52639e539821426c443626880f4edeb1c057dae657cd9b`;
- OpenAPV decoder:
  `4db9c098f1d0cbd60b6614a166aab3dce81ad534ca01c80d1f17c3aae77e9553`;
- benchmark harness:
  `5b49c0261e8da7656914c3c5c75caf2518e5196ff406555fa3985038e4ef86f0`;
- graph generator:
  `1511b8a080c71e9d2c525fcdab9b1a58a7a6e4a888c116b8bc0f61725eae7424`.

The Fastvid binary hashes are retained in `frontier.json` and were checked by
the harness before measurement. The host was a four-vCPU AMD EPYC-Genoa VM
using Rust 1.97.1, CMake 4.2.3, and GCC 15.2.0; the OpenAPV build enabled its
detected SSE4.1 and AVX2 paths.

## Decision

Accept the separate external-reference panel and its methodology. Do not add
OpenAPV to the internal 8-bit aggregate or claim q100 equivalence. Use the
measured encode gap as a high-bit optimization target, and require broader
natural and production 10-bit content before generalizing the compression
result.

## References

- [Research 0011](../research/0011-openapv.md)
- [Research 0015](../research/0015-openapv-matched-comparison.md)
- [EXP-0031](EXP-0031-openapv-matched-baseline.md)
- [EXP-0061](EXP-0061-three-version-frontier.md)
- [EXP-0069](EXP-0069-four-state-frontier-promotion.md)
