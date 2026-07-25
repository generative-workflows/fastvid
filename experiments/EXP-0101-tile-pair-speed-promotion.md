# EXP-0101 — Tile-pair speed-frontier promotion

Status: **ACCEPTED**

## Classification

**Frontier confirmation** — promote accepted EXP-0099 and compare it with the
checksum-pinned OpenAPV reference without rerunning OpenAPV.

## Hypothesis

EXP-0099's byte-identical +19.985% native 10-bit q90 corpus encode gain should
move Fastvid past OpenAPV `fastest` QP23 at the matched one-thread diagnostic,
while preserving Fastvid's bitrate, quality, decode, and access advantages.

## Test

1. Confirm q90/q100 10-bit single-frame access against EXP-0095.
2. Pin source `7525dc2` and the accepted EXP-0099 release binary in
   `frontier.json`.
3. Run six fresh trials for every Fastvid role at q90/q100 and one/four
   threads.
4. Validate and append the established OpenAPV matrix. Do not invoke the
   external encoder or decoder.
5. Regenerate the matched and internal frontier summaries/graphs.

## Gate

- Fastvid speed q90 one-thread encode exceeds OpenAPV `fastest` at its closest
  measured PSNR point;
- speed bytes, quality, dependency counts, and access amplification are
  unchanged;
- access latency and decode stay within 5%;
- speed remains internally non-dominated; and
- the OpenAPV result hash remains exactly pinned.

## Result

The matched run measured 72 fresh Fastvid rows and appended 144 OpenAPV rows
that are field-for-field identical to the pinned matrix. The validated
OpenAPV artifact hash remains
`b48462cea78b6c51d8d8ae2e51dd0b640b8308f47265a4467facc4b0825405b0`;
no OpenAPV encoder or decoder process ran.

One-thread q90-neighborhood medians:

| Codec | Control | Ratio | Encode | Decode | Bitrate | Y PSNR |
|---|---:|---:|---:|---:|---:|---:|
| Fastvid speed | q90 | 4.809339x | 93.787 MP/s | 68.387 MP/s | 147.169656 Mb/s | 52.001930 dB |
| OpenAPV fastest | QP23 | 4.464067x | 81.182 MP/s | 63.471 MP/s | 158.552448 Mb/s | 51.735588 dB |

Fastvid is 15.53% faster to encode, 7.75% faster to decode, uses 7.18%
less playback bitrate, and has 0.266 dB higher Y-PSNR. At four threads it
measures 220.073/160.341 MP/s encode/decode versus OpenAPV's
218.557/134.888: +0.69% encode and +18.87% decode.

At the distinct high-fidelity boundary, exact Fastvid q100 measures
63.843 MP/s and 257.969880 Mb/s. OpenAPV `fastest` QP0 measures
63.200 MP/s and 360.180000 Mb/s but is not exact (`max_error=2`).
Fastvid is 1.02% faster and uses 28.38% less bitrate at the higher-fidelity
boundary.

The internal four-case screening frontier remains non-dominated:

| Slot | Ratio | Encode | Decode |
|---|---:|---:|---:|
| Speed | 13.353556x | 122.926 MP/s | 149.193 MP/s |
| Practical compression | 24.547776x | 29.634 MP/s | 139.090 MP/s |
| Maximum compression | 33.588694x | 25.248 MP/s | 103.861 MP/s |

Balanced q90/q100 10-bit access confirmation against EXP-0095 found:

| Quality | Access latency | Useful MP/s | Work MP/s |
|---:|---:|---:|---:|
| 90 | -0.447% | +0.449% | +0.449% |
| 100 | -0.837% | +0.844% | +0.845% |

Encoded bytes read, dependency frames, decoded frames, and access
amplification are identical.

Artifacts:

- speed binary:
  `artifacts/frontier/fastvid-speed-exp0099-tile-pairs`
  (`41f5719eb0630cc8dd78067806dfe4775b30d9e3b9b59e0701775d40c91e71af`);
- matched raw matrix:
  `artifacts/exp0101-openapv-frontier.tsv`
  (`93de04f03c8d5be5d7869e91c2f8dc580660d3473c7369462e310191b305a060`);
- internal raw matrix:
  `artifacts/exp0101-frontier-fast-feedback.tsv`
  (`fa483243165a02c08239b2c8b4273a389e8cc9efdea6430a68b3bccfeb4f4360`);
- access matrix:
  `artifacts/exp0101-speed-access.tsv`
  (`69c2eb8807302c68af467aa3481c9984e56a12d3cb27e8c1dd5433bdca8ce93e`);
- matched graph:
  `benchmarks/openapv-frontier.svg`
  (`84e500e6b3f5e949085e0507775fda2828e652ef475d670b9acfee009fe6afc2`);
- matched summary:
  `benchmarks/openapv-frontier-summary.tsv`
  (`961dd5a58a2ad2f33c8c04c64603eccdc51be7f396b273fca04b4c3d859b8b8e`);
- internal graph:
  `benchmarks/frontier.svg`
  (`343edca6a80c2f8bbf8ad54dc333809380e2ea026ca1aa541d839bc417023de3`);
- internal summary:
  `benchmarks/frontier-summary.tsv`
  (`7ab84ffa8248a8329993efb683171013aac5daa37214de1ac414657af6afd31f`).

## Decision

Promote source `7525dc2` and binary
`41f5719eb0630cc8dd78067806dfe4775b30d9e3b9b59e0701775d40c91e71af`
to the speed slot. The candidate clears the stated goal of beating OpenAPV
`fastest` on matched, high-quality one-thread encoding without spending rate,
quality, decode speed, or access behavior.

This result is scoped to one procedural 10-bit sequence and is not a broad
natural-HDR superiority claim. The goal remains active: the wider high-bit
corpus needs natural production footage, and the newly defined
parallel-hardware serial-span target is not yet implemented.

## References

- [EXP-0096](EXP-0096-rice4-speed-promotion.md)
- [EXP-0099](EXP-0099-interleaved-rice-tile-pairs.md)
