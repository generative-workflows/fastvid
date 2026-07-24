# EXP-0062 — Speed-tier entropy profile

Status: **ACCEPTED**

## Classification

**Exploitation diagnosis** — re-profile after EXP-0060 removed exhaustive
predictor evaluation, before choosing an entropy-path optimization.

## Hypothesis

On the fixed-gradient speed tier, residual accumulation plus final entropy
writing will account for at least 25% of exclusive encode samples because the
former Paeth and multi-predictor costs are absent. Repeated PMU counts should
show whether the remaining cost is instruction-, branch-, or memory-load
heavy.

## Modification

No codec change. Profile the exact preserved EXP-0060 binary and a
source-mapped build reproduced from commit `4ad0318` plus
`artifacts/frontier/exp0060-speed.patch`.

Use the same q90, GOP-1, one-thread, 24-frame noisy-camera workload as
EXP-0058:

- 999 Hz `perf record` call-graph sampling;
- five `perf stat` trials for cycles, instructions, branches, branch misses,
  and L1D loads;
- exact binary-hash validation.

## Test and gate

Accept as actionable if a coherent residual/entropy kernel accounts for at
least 25% of exclusive encode samples and PMU trial spread is below 3%.
Otherwise expand to Cachegrind or a temporal case before selecting a change.

## References

- [Research 0012](../research/0012-simd-cache-profiling.md)
- [EXP-0058](EXP-0058-frontier-speed-profile.md)
- [EXP-0060](EXP-0060-fixed-gradient-speed-tier.md)

## Result

The exact preserved binary produced 1,983 samples with zero lost samples:

| Exclusive symbol | Samples |
|---|---:|
| encode closure | 34.25% |
| `reconstruct_sample` | 25.11% |
| `decode_tile_payload` | 14.94% |
| `ResidualAccumulator::finish` | 13.74% |

A reproduced source build agreed at symbol level. Because thin LTO collapsed
line tables, a supplemental non-LTO debuginfo build was used only to separate
the source structure, not for timing conclusions. It attributed 30.22% to
`encode_spatial_tile` and 11.62% to `ResidualAccumulator::finish`, a coherent
**41.84%** residual-construction/finalization hot set. Rice decoding was also
visible at 21.52% in that diagnostic build.

Five exact-binary PMU trials averaged:

| Counter | Count |
|---|---:|
| cycles | 6,590,596,193 |
| instructions | 24,330,614,703 |
| branches | 3,529,701,904 |
| branch misses | 72,386,099 |
| L1D loads | 7,625,035,862 |

IPC was 3.69 and branch-miss rate was 2.05%. Maximum relative spread was
0.24%, below the 3% gate. Compared with EXP-0058 balanced, speed executes fewer
instructions and cycles but essentially the same number of L1D loads. This is
consistent with residual buffering/histogram traffic surviving the predictor
simplification.

Artifacts:

- exact profile: `artifacts/exp0062-speed-perf.data`
  (`be9ce4830d7c4296678bf226b6f432466b1effaec65715fea548bb6241dad05e`);
- thin-LTO source profile: `artifacts/exp0062-speed-source-perf.data`
  (`b70defdd7f55b422ba73656d56b6de1bd9c8e5f8b6ff372e0c9b616bf68e1d7c`);
- no-LTO source diagnostic:
  `artifacts/exp0062-speed-source-nolto-perf.data`
  (`c5cc18fe207daf5025c194753e28617fd028f841c3ee53cefa69a7493569f435`);
- repeated PMU counts: `artifacts/exp0062-speed-stat.tsv`
  (`aef5e0c5d057a7ba74a07e23dd376893808b6ad0b4f5c213b3fc18592a8e245f`).

## Decision

**Accepted as actionable.** The sample and PMU gates pass. The next speed
experiment should avoid accumulating and rescanning every residual: estimate
the existing zero-run/Rice mode from a sparse causal proxy, then stream the
chosen existing entropy syntax during the single reconstruction pass.
