# EXP-0096 — Rice-4 speed-frontier promotion

Status: **ACCEPTED**

## Classification

**Frontier confirmation** — compare accepted EXP-0095 against pinned
Fastvid roles and the checksum-pinned OpenAPV reference.

## Hypothesis

EXP-0095's complete q90 improvement should survive a fresh matched frontier
run and reduce the one-thread encode deficit to OpenAPV `fastest` QP23 from
12.18% to below 8%, while retaining Fastvid's bitrate and decode advantages.

## Test

1. Pin source `91a755e` and the accepted EXP-0095 release binary in
   `frontier.json`.
2. Run six fresh trials for every Fastvid role at q90/q100 and one/four
   threads.
3. Checksum-validate and append the established OpenAPV matrix; do not rerun
   the fixed external codec.
4. Regenerate the internal and OpenAPV frontier summaries/graphs.
5. Update current-state documentation only after validating row counts,
   hashes, quality matching, bitrate, and throughput.

## Gate

- matched q90 one-thread encode deficit to OpenAPV below 8%;
- identical EXP-0088 bytes and quality at q90/q100;
- speed role remains non-dominated internally;
- OpenAPV reference rows are byte-for-byte reused from the pinned artifact;
- graphs and summaries validate against their raw matrices.

## Result

The matched external-reference run measured 72 fresh Fastvid rows and reused
144 OpenAPV rows after validating the pinned result hash
`b48462cea78b6c51d8d8ae2e51dd0b640b8308f47265a4467facc4b0825405b0`.
No OpenAPV encoder or decoder process ran.

One-thread q90-neighborhood medians were:

| Codec | Control | Ratio | Encode | Decode | Bitrate | Y PSNR |
|---|---:|---:|---:|---:|---:|---:|
| Fastvid speed | q90 | 4.809339x | 76.255 MP/s | 69.115 MP/s | 147.169656 Mb/s | 52.001930 dB |
| OpenAPV fastest | QP23 | 4.464067x | 81.182 MP/s | 63.471 MP/s | 158.552448 Mb/s | 51.735588 dB |

Fastvid's encode deficit is now 6.07%, below the 8% gate and down from the
previous 12.18%. Equivalently, OpenAPV is 6.46% faster. Fastvid uses 7.18%
less bitrate, has 0.266 dB higher Y-PSNR, and decodes 8.89% faster.

At four threads Fastvid measured 196.718 MP/s encode and 156.946 MP/s decode,
versus OpenAPV's 218.557 and 134.888 MP/s. The remaining encode deficit is
9.99%, while Fastvid decode is 16.35% faster.

At the distinct high-fidelity boundary, exact Fastvid q100 measured
66.036 MP/s encode at 257.969880 Mb/s. Non-exact OpenAPV `fastest` QP0
measured 63.200 MP/s with maximum error 2 at 360.180000 Mb/s. Fastvid is
4.49% faster and uses 28.38% less bitrate; this is not presented as a
nominal-control match.

The internal four-case matrix retained three non-dominated roles:

| Slot | Ratio | Encode | Decode |
|---|---:|---:|---:|
| Speed | 13.353556x | 109.433 MP/s | 149.527 MP/s |
| Practical compression | 24.547776x | 30.077 MP/s | 137.631 MP/s |
| Maximum compression | 33.588694x | 25.113 MP/s | 103.703 MP/s |

The fresh speed rows retained EXP-0088's exact bytes and metrics. Both graph
generators validated their expected matrices and trial counts.

Artifacts:

- matched raw matrix:
  `artifacts/exp0096-openapv-frontier.tsv`
  (`d4f99f546d080bf8d96be1f0710fd195befec1d9d43945f43ad7d599bd3d31af`);
- internal raw matrix:
  `artifacts/exp0096-frontier-fast-feedback.tsv`
  (`7ccd7d79e710249ad63d0e17c392a52dcfed3e00022b66c04efb10db1828b7a3`);
- matched graph:
  `benchmarks/openapv-frontier.svg`
  (`ddf55fc053579402274a8cba10fa525c5eb9b28a0392ee198a32e2b3152063ff`);
- matched summary:
  `benchmarks/openapv-frontier-summary.tsv`
  (`d002d69ece1a68a7f1407693b0e6707f64edf02665ec97986856cf4154924703`);
- internal graph:
  `benchmarks/frontier.svg`
  (`4a5812d4e64d6cc85b67ca1d87021581f9a559baef0198142b3d02768e6ef1a4`);
- internal summary:
  `benchmarks/frontier-summary.tsv`
  (`1c105b1069b8fd43da0e8ab992f79851e23e7a910201b3cf7aa6e5b0e9e419f7`).

## Decision

Promote source `91a755e` and binary
`637ee0535510f38dd9dc99f02fc5acbd75f7d927f5b2a3517d2b8f4b167c1407`
to the speed slot. It remains internally non-dominated and materially closes
the principled q90 OpenAPV target without changing compression or quality.

The primary goal remains open: Fastvid has not yet beaten OpenAPV `fastest`
at the q90-neighborhood operating point. The next exploitation target needs
approximately another 6.5% relative encode improvement.

## References

- [EXP-0089](EXP-0089-portable-kernel-speed-promotion.md)
- [EXP-0095](EXP-0095-block-pack-rice4-combination.md)
