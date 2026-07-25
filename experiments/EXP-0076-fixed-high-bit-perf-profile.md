# EXP-0076 — Fixed high-bit branch perf profile

Status: **ACCEPTED**

## Classification

**Speed exploitation diagnosis** — profile the strongest branch from
EXP-0074 before choosing its next implementation optimization.

## Hypothesis

After removing multi-predictor search, the remaining q90 high-bit encode time
is dominated by one or more of:

- fixed clamp-gradient residual construction and reconstructed-row updates;
- repeated Rice parameter reductions over the folded tile vector;
- zero-run/Rice payload sizing and emission;
- per-tile allocation or thread/output coordination.

A sampled profile plus hardware counters will identify a dominant kernel
whose removal or fusion plausibly closes a material part of the remaining
1.64x gap to OpenAPV `fastest`.

## Measurement

Profile the preserved EXP-0074 candidate and its commit-`789ab97` baseline on
the checksummed 1280x720x24 native-10-bit sequence at q90, GOP 1, 256x128
tiles, and one thread:

1. warm each binary once;
2. collect repeated `perf stat` cycles, instructions, branches,
   branch-misses, cache references, and cache misses when the kernel permits;
3. collect call-graph samples with `perf record`;
4. retain raw reports and binary hashes;
5. compare samples only within each binary because the fixed branch changes
   the work decomposition.

If hardware events are unavailable, record the exact permission failure and
fall back to software-clock sampling. Do not infer cache behavior from wall
time alone.

## Decision gate

Advance one implementation direction only when:

- the candidate profile attributes at least 15% of samples or cycles to a
  concrete optimizable kernel; and
- the proposed change can preserve the candidate's stream, quality, and
  decoder behavior, or explicitly declares a separate frontier tradeoff.

The profile itself changes no production source.

## Result

Hardware counters were available without privilege escalation. Five candidate
trials had sub-1% relative spread and measured:

| Counter | Exhaustive baseline | Fixed branch | Change |
|---|---:|---:|---:|
| cycles | 6,663,249,464 | 3,470,063,171 | -47.92% |
| instructions | 27,237,718,830 | 13,608,018,268 | -50.04% |
| branches | 4,194,032,147 | 2,399,741,806 | -42.78% |
| branch misses | 58,795,116 | 28,085,559 | -52.23% |
| cache references | 189,428,444 | 102,599,101 | -45.84% |

These counters cover the CLI's encode, decode, and metric work, but both
binaries use the same boundary. They independently support EXP-0074's large
wall-time reduction. The generic `cache-misses` event returned zero for both
binaries despite substantial cache references, so it is excluded as an
invalid host event rather than interpreted.

The 999 Hz fixed-branch profile captured 996 cycle samples with none lost.
Largest exclusive symbols were:

| Symbol | Whole-benchmark samples |
|---|---:|
| `codec16::finish_entropy` | 25.58% |
| fixed high-bit encode closure | 18.79% |
| `codec16::decode_tile_payload` | 17.19% |
| `codec16::reconstruct` | 12.85% |
| `metrics::compare_plane16` | 4.64% |
| `metrics::ssim_plane16_sampled` | 4.13% |

Encoding is approximately the 44.37% combination of residual construction
and entropy finalization in this mixed command, making `finish_entropy` about
58% of the visible encode hot set. Annotation attributes substantial samples
to the repeated parameter-outer Rice reductions, the zero-run sizing walk,
and Rice bit emission.

The first encoded frame contained 45 tiles: 15 zero-run tiles, 15 Rice
parameter-0 tiles, and 15 Rice parameter-4 tiles. A universal fixed entropy
mode is therefore not justified even on this one diagnostic.

Artifacts:

- fixed-branch counters:
  `artifacts/exp0076-candidate-perf-stat.txt`
  (`04df64e0bb76629a6f638eb3976a68f82b4e71cd8476b5969ff17970e53da4e9`);
- baseline counters:
  `artifacts/exp0076-baseline-perf-stat.txt`
  (`952845e3ab14ad57131deebfc578fc8d905446353494577739dc69648b2e3c18`);
- sampled profile:
  `artifacts/exp0076-candidate-perf.data`
  (`43cfc23b62ff18fd281bed375c9f95ac2353433786b6264084c8a3fbc3d7aaa0`);
- text report:
  `artifacts/exp0076-candidate-perf-report.txt`
  (`39804fda6c658d789e531f71f924f41a58e0b1c360fb7a0da79193c05ae363d9`).

The baseline and candidate hashes remain those recorded in EXP-0074.
Restricted kernel symbol maps produced unresolved kernel samples, but all
named Fastvid userspace hotspots resolved; no kernel behavior is inferred.

## Decision

Accept as actionable diagnostic evidence. Advance an entropy-finalization
optimization for the fixed high-bit branch. It must avoid the 17-accumulator
inner-loop failure from EXP-0050 and the universal second prepass rejected by
EXP-0063. A small deterministic prefix estimator followed by direct emission
is the highest-confidence next screen; exact fallback remains necessary when
the prefix chooses zero-run or has low confidence.

## References

- [Research 0012](../research/0012-simd-cache-profiling.md)
- [Research 0014](../research/0014-sampling-and-high-bit-quantization.md)
- [Research 0019](../research/0019-modern-integer-entropy-kernels.md)
- [EXP-0034](EXP-0034-perf-samply-cache-profile.md)
- [EXP-0074](EXP-0074-fixed-predictor-high-bit-speed.md)
