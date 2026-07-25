# EXP-0074 — Fixed-predictor high-bit speed branch

Status: **REJECTED**

## Classification

**Exploitation for the speed frontier** — remove known high-bit encoder
search work before adding new coding tools. This is a deliberately distinct
rate/throughput branch, not a proposed replacement for maximum compression.

## Hypothesis

The current high-bit intra encoder constructs, reconstructs, and
entropy-scores Paeth, average, and clamp-gradient residuals for every tile.
Directly encoding one fixed clamp-gradient candidate should improve
one-thread q90 encode throughput by at least 2x on the matched OpenAPV
diagnostic while preserving reconstruction metrics exactly. Based on
EXP-0047 and EXP-0060, a spatial payload increase is expected and must remain
visible rather than being treated as a regression-free optimization.

This first exploitation may not by itself reach OpenAPV `fastest` at 80.724
MP/s. It is useful only if it materially closes the measured 4.88x gap
without destroying the bitrate advantage.

## Modification

Starting from commit `789ab97`:

- when the frame-level activity gate selects temporal prediction, directly
  encode the temporal residual;
- otherwise directly encode clamp-gradient residuals and signal the existing
  version-2 clamp-gradient mode;
- skip all per-tile high-bit predictor candidate vectors, reconstructed
  rows, squared-error accumulation, entropy estimates, and selection;
- retain every decoder mode and leave the 8-bit path unchanged;
- preserve the candidate as a reproducible source patch and release binary,
  not as the maximum-compression working-tree implementation.

No tile geometry, quality mapping, entropy syntax, thread implementation, or
decoder behavior changes.

## Fast test

Use the checksummed 1280x720, 24-frame native-10-bit sequence from EXP-0073
at q90/q100, GOP 1, 256x128 tiles, and one/four threads:

1. build distinct baseline and candidate release binaries;
2. warm both and run six serial trials in alternating order;
3. require deterministic encoded bytes and reconstruction metrics;
4. report ratio, bitrate, encode/decode MP/s, Y-PSNR, SSIM, and maximum error;
5. compare the q90 medians with the measured OpenAPV `fastest` QP23 point.

If the fast gate passes, confirm q90/q100 on the complete native high-bit
supplement and run native high-bit single-frame access before promoting the
branch.

## Gate

- at least 2x one-thread q90 encode throughput versus the baseline;
- no quality, maximum-error, or q100 exactness change;
- no more than 20% q90 encoded-byte increase on the focused sequence;
- no decode regression beyond the 5% timing tolerance;
- a distinct non-dominated speed/rate point after focused confirmation.

Beating OpenAPV `fastest` is recorded as the project target, not weakened
into this experiment's acceptance gate. Failure to reach 80.724 MP/s informs
the next optimization rather than invalidating a substantial independent
speed-frontier improvement.

## Result

The 48-row focused matrix completed six alternating serial trials for every
variant/quality/thread cell. Encoded bytes and reconstruction metrics were
stable within every cell.

| Quality | Threads | Variant | Ratio | Encode | Decode | Bitrate |
|---:|---:|---|---:|---:|---:|---:|
| 90 | 1 | baseline | 5.307903x | 16.663 MP/s | 59.778 MP/s | 133.346224 Mb/s |
| 90 | 1 | fixed gradient | 4.685392x | 49.319 MP/s | 63.472 MP/s | 151.062880 Mb/s |
| 90 | 4 | baseline | 5.307903x | 58.978 MP/s | 160.928 MP/s | 133.346224 Mb/s |
| 90 | 4 | fixed gradient | 4.685392x | 149.952 MP/s | 151.826 MP/s | 151.062880 Mb/s |
| 100 | 1 | baseline | 2.949766x | 17.831 MP/s | 56.140 MP/s | 239.947400 Mb/s |
| 100 | 1 | fixed gradient | 2.744292x | 50.689 MP/s | 58.395 MP/s | 257.913104 Mb/s |
| 100 | 4 | baseline | 2.949766x | 63.740 MP/s | 156.228 MP/s | 239.947400 Mb/s |
| 100 | 4 | fixed gradient | 2.744292x | 154.886 MP/s | 152.471 MP/s | 257.913104 Mb/s |

The candidate improved one-thread encoding by 2.96x at q90 and 2.84x at
q100, confirming that exhaustive predictor construction and scoring is the
dominant high-bit encode cost. It remained 1.64x slower than the matched
OpenAPV `fastest` QP23 result of 80.724 MP/s.

At q90, encoded bytes increased 13.29%. Y-PSNR changed from 52.002293 to
52.001930 dB and luma block SSIM from 0.99373118 to 0.99373056; maximum error
remained 4. The differences are negligible in magnitude but violate the
predeclared exact quality-invariance gate. Four-thread q90 decode throughput
also regressed 5.66%, just outside the 5% timing tolerance. q100 remained
exact.

The release suite had 52/54 library tests pass. The two failures were
selector-policy assertions:

- `high_bit_legacy_version_accepts_only_legacy_modes` expected the production
  encoder to choose legacy Paeth, while the isolated branch signals the
  existing version-2 clamp-gradient mode;
- `predictor_oracle_matches_current_high_bit_payloads_and_error_bound`
  expected production output to equal the exhaustive oracle.

Generic q100 round-trip, lossy error-bound, independent mode-decode, malformed
stream, entropy, metric, and 8-bit tests passed. The failures are nevertheless
retained as failed acceptance evidence rather than weakening
maximum-compression tests for a separate branch.

Artifacts:

- raw focused matrix:
  `artifacts/exp0074-fixed-highbit-focused.tsv`
  (`bfc3b521299dd14043f180067108080da2e444bf1868dcbc6f6250c0563674ed`);
- source patch:
  `artifacts/frontier/exp0074-fixed-highbit-speed.patch`
  (`13a856850635d4189e1913568deb6faedcb1bf0d1676c5d7dc0e560f6d66a654`);
- baseline binary:
  `5eeea24df59f7750f48d27cd399cfe6cebb958a98aba9fd7e498a2890fdb69a9`;
- candidate binary:
  `artifacts/frontier/fastvid-highbit-speed-exp0074`
  (`0f6c2e5b8a1761595a284fc830a5084c4e3112dc64c1b8745017235178d406d6`);
- benchmark harness:
  `00f29c0e19e632c01ff2f4b9ce55cbbc9d98f780b0ce0b7de309958a3ccfbce0`;
- summary validator:
  `c8f2a1eb3833f086c4eb93c252411bad43b74a9f83254a7daf82443074dfbb32`.

## Decision

Reject under the strict gate and restore the maximum-compression working
source. Preserve the candidate as a measured exploration branch: its nearly
3x encode gain is strong evidence for a distinct high-bit speed frontier, but
promotion requires a successor experiment with explicit principled
rate/quality tolerances, complete native high-bit confirmation, access
measurement, and tests scoped to the branch's declared predictor policy.

## References

- [Research 0026](../research/0026-paeth-data-dependency-kernel.md)
- [EXP-0047](EXP-0047-compatible-predictor-oracle.md)
- [EXP-0051](EXP-0051-high-bit-staged-predictors.md)
- [EXP-0060](EXP-0060-fixed-gradient-speed-tier.md)
- [EXP-0073](EXP-0073-matched-openapv-frontier.md)
