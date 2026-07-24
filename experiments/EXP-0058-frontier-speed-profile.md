# EXP-0058 — Frontier speed profile

Status: **ACCEPTED**

## Classification

**Exploitation diagnosis for the vacant speed role** — compare the preserved
balanced and practical-compression implementations under the same sampling
and PMU protocol before selecting another optimization target.

## Hypothesis

The balanced line's remaining one-thread encode cost is concentrated in
residual construction/entropy coding rather than tile coordination, while the
practical line additionally spends substantial time evaluating compatible
predictors. A symbolized profile will identify a kernel responsible for at
least 15% of on-CPU samples and supported by repeated instruction/cache
counters.

## Modification

No codec or bitstream change. Profile the exact preserved frontier binaries:

- balanced `06ef3278…ab8`;
- practical compression `1235c7e8…31c9`.

Use the q90 24-frame 1920x1080 noisy-camera clip at GOP 1, one thread, and
default tiles after one untimed warm-up. This supplies enough on-CPU duration
for sampling without mixing temporal prediction into the first diagnosis.
Record:

- 999 Hz `perf` call-graph samples;
- five `perf stat` trials for cycles, instructions, branches, branch misses,
  L1D loads, and L1D load misses; and
- Cachegrind only for the strongest candidate kernel if PMU and sampling
  attribution disagree.

The benchmark command times encode/decode internally; whole-process PMU totals
are diagnostic and are not substituted for codec throughput.

## Test

- Validate binary hashes before every profile.
- Reject unsupported PMU aliases or implausible zero counts.
- Preserve raw perf data and exact commands.
- Compare exclusive symbols rather than attributing all child work to a broad
  caller.
- Select a follow-up only when the profile and source structure agree.

## Gate

Accept the profile as actionable if one implementation kernel accounts for at
least 15% of exclusive samples, or a small coherent set accounts for at least
30%, and repeated PMU counts have less than 3% relative spread. Otherwise
expand profiling to a video and four-thread case before modifying code.

## References

- [Research 0012](../research/0012-simd-cache-profiling.md)
- [Research 0014](../research/0014-sampling-and-high-bit-quantization.md)
- [EXP-0034](EXP-0034-perf-samply-cache-profile.md)
- [EXP-0057](EXP-0057-automated-pareto-frontier.md)

## Environment

- CPU: AMD EPYC (Genoa), four available cores, AVX2 and AVX-512
- `perf`: 7.0.12
- `perf_event_paranoid`: 1
- Corpus case: `noisy-camera-1920x1080-24f-q90-g1`
- Threads: 1

The exact preserved binaries were profiled first. Supplemental binaries built
from the corresponding commits with release debuginfo supplied source-line
attribution; their symbol-level distributions agreed with the preserved
binaries.

## Results

| Line | Bytes | Ratio | Encode ms | Encode MP/s | Decode ms | Decode MP/s |
|---|---:|---:|---:|---:|---:|---:|
| balanced | 32,630,454 | 3.050304x | 1,343 | 37.048 | 945 | 52.616 |
| practical | 32,491,767 | 3.063324x | 3,516 | 14.156 | 925 | 53.797 |

The principal exclusive samples were:

| Line | Kernel | Samples |
|---|---|---:|
| balanced | encode closure | 40.72% |
| balanced | reconstruct sample | 24.60% |
| balanced | residual accumulator finish | 14.34% |
| practical | encode closure | 76.67% |
| practical | `encode_best_tile` generic predictor evaluation | 26.14% |
| practical | residual accumulator push | 10.59% |

Within the balanced encode closure, Paeth selection and its absolute-distance
work accounted for about 17.81% of total samples. Within the practical encode
closure, the residual accumulator, quantizer, spatial prediction, Paeth, Rice
writer, indexing, and allocation costs form a coherent predictor-evaluation
hot set.

Five whole-process PMU trials produced:

| Line | Cycles | Instructions | IPC | Branch misses | Branch-miss rate | L1D loads |
|---|---:|---:|---:|---:|---:|---:|
| balanced | 8,080,232,094 | 26,989,348,721 | 3.34 | 92,745,955 | 2.64% | 7,571,894,201 |
| practical | 15,577,464,886 | 56,063,811,460 | 3.60 | 164,240,983 | 2.29% | 18,950,651,656 |

Relative trial spread was at most 0.54%, below the 3% gate. Compared with
balanced, practical consumed 1.93x cycles, 2.08x instructions, and 2.50x L1D
loads. The requested L1D-load-miss alias reported zero and was rejected as an
unsupported/invalid event rather than interpreted.

## Artifacts

- `artifacts/exp0058-balanced-perf.data`
  (`1930bf021761440bc9c3d83e9b9496071f935719a253b15446df6afda466d471`)
- `artifacts/exp0058-practical-perf.data`
  (`d09366f90f43527acfe2d4dc07fc0885409c861e549a24006e153ac3750c431d`)
- `artifacts/exp0058-balanced-source-perf.data`
  (`50d63bd4b655a8b48c9bf035b5fca153a148929101fd575c3061e211e4300f93`)
- `artifacts/exp0058-practical-source-perf.data`
  (`ebb074345e300805dcc8597e35120111f351c4cd41ea8f29f9cc638e5980deee`)
- `artifacts/exp0058-balanced-stat.tsv`
  (`29ae16f4876745982239d5092e8803e5c60b7dde6c91ecbba3065a4d0be39241`)
- `artifacts/exp0058-practical-stat.tsv`
  (`7ca60f2749ee6c7864191fca13f3c6d812c1b06c8dd919a1f7932863f39863a`)

## Decision

**Accepted.** The sampling and PMU gates both passed and agree with the source
structure. The next speed experiments should target:

1. byte-identical Paeth selection on the balanced line; and
2. staged or sampled predictor selection on the practical line, avoiding the
   current full-image repeated candidate work.

Tile-result coordination is not a measured hot path and is excluded from the
next optimization round.
