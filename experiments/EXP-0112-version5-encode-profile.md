# EXP-0112 — Version-5 encode profile

Status: **ACCEPTED**

## Classification

**Measurement / exploration** — identify the accepted version-5 encoder's
actual CPU bottlenecks before the next optimization.

## Hypothesis

On a repeated native 10-bit q90 encode, exact 17-parameter four-lane Rice
selection and emission account for a majority of sampled version-5 encoder
CPU time. Predictor/quantizer work should be the second major component;
allocation itself should be minor, consistent with EXP-0111.

## Modification

Extend the existing encode-only profiling harness to select the version-5
entry point. Record `perf` software-clock call graphs and available hardware
counters on a repeated fixed corpus frame. Do not change codec behavior.

## Gate

- profile only encoding, outside file input and frame construction;
- record exact command, host counter availability, samples, and top symbols;
- use the evidence to select or reject the next optimization direction;
- profiling harness builds under strict Clippy.

## Result

The encode-only harness ran the 1920x1080 10-bit HDR q90 frame 20 times with
one thread and default 256x128 tiles:

```text
perf stat -e task-clock,cycles,instructions,branches,branch-misses,\
cache-references,cache-misses -- \
target/release/encode16_profile \
artifacts/corpus-v2/native/hdr-gradient-1920x1080-yuv422p10le.raw \
1920 1080 1 10 90 256 128 20 bounded-full-tile
```

It measured 3,551.82 ms task-clock, 12.523 billion cycles, 40.734 billion
instructions, 7.685 billion branches, and 34.346 million branch misses. This
is about 3.25 instructions/cycle, a 0.447% branch-miss rate, and 11.68 MP/s.
The kernel exposed hardware counters at `perf_event_paranoid=1`. It reported
51.664 million cache references but zero cache misses; that zero is not
credible on this host and is not used as optimization evidence.

A separate 30-repeat `perf record -e cycles:u -g --call-graph dwarf` captured
23K samples with zero lost samples. Self-time:

| Symbol | Cycles |
|---|---:|
| `encode_parallel_rice` parameter-search closure | 68.34% |
| version-5 tile encoder closure | 17.16% |
| `BitWriter::put_rice` | 9.55% |
| `put_fixed_block` | 1.14% |
| frame validation | 0.77% |
| AVX-512 `memmove` | 0.65% |

Rice search plus emission therefore accounts for 77.89% of sampled cycles.
No allocator symbol is material. This independently explains EXP-0111's
failure: allocation was not the bottleneck.

Artifacts:

- `artifacts/exp0112-perf-stat.tsv`
  (`87bff9773b00402c93c000a552bac5ace6432f12e9552af89b829ba86359578b`);
- `artifacts/exp0112-v5-encode-perf.data`
  (`ed56330da4d00a3b1e13e6ef7f70913d2da655e096f3a5ca07ce246c72cde55a`);
- `artifacts/exp0112-v5-encode-perf-report.txt`
  (`3be81f141902192a27ce52681230bdcd36ea05790d6ca010443f953d645bfe95`).

## Decision

Accept the profile. Target exact Rice parameter search before allocation,
block packing, cache layout, or explicit SIMD.

The first exploitation branch should stop the parameter scan once every
lane's quotient sum is zero. At that point all higher parameters have the
same zero quotient and strictly greater remainder cost, so they cannot win,
including after per-lane byte rounding. This is byte-identical and should
roughly halve search work on bounded 10-bit q90 residuals. A later exploration
branch can evaluate a one-pass histogram or mean-guided narrow search if
exact early termination is insufficient.

## References

- [Research 0012](../research/0012-simd-cache-profiling.md)
- [Research 0014](../research/0014-sampling-and-high-bit-quantization.md)
- [EXP-0110](EXP-0110-full-tile-bounded-shards.md)
- [EXP-0111](EXP-0111-winner-only-shard-emission.md)
