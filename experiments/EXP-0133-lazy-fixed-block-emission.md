# EXP-0133 — Lazy fixed-block emission

Status: **ACCEPTED**

## Classification

**Version-5 emission exploitation** — remove losing candidate construction
from the post-EXP-0130 hot path without changing the bounded-shard format.

## Hypothesis

EXP-0130's cycle profile attributes 4.61% self time to fixed-block body
emission and exposes allocator/free work beneath it. The selector already
computes the exact complete fixed-block body length before emission, but the
encoder constructs that body for every shard even when zero-run or Rice has
already won.

Constructing fixed blocks only when their exact modeled length is strictly
smaller than both competing bodies should:

- preserve the existing zero-run/Rice/block tie order and every stream byte;
- remove all losing fixed-block allocations and writes;
- improve geometric whole-codec v5 encode throughput by at least 1.03x;
- retain at least 0.95x encode throughput on every native sample; and
- retain at least 0.95x geometric decode throughput.

The 3% screening gate is lower than the normal 5% complete-binary promotion
gate because the profile places a hard upper bound near 4.6%. Acceptance
still requires repeated balanced native-corpus evidence and exact bytes.

## Modification

In `encode_parallel_shard_with_block_pack`:

1. compute the exact fixed-block cost;
2. compare it with the already exact Rice length and fused zero-run length;
3. construct the fixed-block body only when it is strictly smaller than both;
4. otherwise emit the unchanged zero-run/Rice winner.

Do not alter prediction, residual order, entropy costs, tie behavior, shard
syntax, or output assembly.

## Test

- retain the block-pack cost/emission oracle and v5 round-trip tests;
- retain the EXP-0130 HDR control hash and every native q90 stream byte;
- screen with balanced fast feedback against the exact EXP-0130 binary;
- if the 3% gate is plausible, run five balanced native-corpus trials;
- run the full release suite, strict Clippy, formatting, shell syntax, and
  diff checks if accepted;
- revert the implementation if the complete-binary gate fails.

## Result

The exact block-pack cost remains equal to emitted length in the existing
oracle, and the v5 round-trip/select tests remain green. The candidate
retains the EXP-0130 HDR control SHA-256
`9a3cf708ecdc73f9f8c15a545b41f761ad1ed844c2b8cb4db42118ce587fce37`.
Every native q90 encoded byte, ratio, bitrate, quality metric, and maximum
error is unchanged.

Five balanced whole-codec trials measured:

| Sample | Candidate encode | Encode ratio | Decode ratio | Encoded bitrate |
|---|---:|---:|---:|---:|
| HDR gradient 10 | 42.402 MP/s | 1.050x | 1.024x | 333.288000 Mb/s |
| Precision motion 10 | 48.555 MP/s | 1.094x | 1.034x | 148.023112 Mb/s |
| Precision UI 12 | 50.979 MP/s | 1.163x | 1.055x | 229.381632 Mb/s |
| Precision motion 16 | 66.862 MP/s | 1.129x | 1.023x | 38.988880 Mb/s |
| **Geometric** | — | **1.1080x** | **1.0342x** | — |

Every sample passes the per-case encode gate, and the geometric encode gate
passes by a wide margin. The improvement is larger than the 4.61% direct
fixed-block symbol because the old losing path also allocated, freed, and
copied candidate storage beneath that symbol.

The profiling-feature HDR run measured 1,397.17 ms task-clock, 4.917 billion
cycles, 20.454 billion instructions, 2.897 billion branches, and 45.384
million branch misses over 30 encodes: about 4.16 instructions/cycle and
44.52 MP/s. Instrumented task clock is unchanged within tolerance from
EXP-0130, while retired work reduces instructions 5.26% and branches 5.16%.
The balanced ordinary binaries are the authoritative speed evidence.

A 60-repeat cycle profile captured 11,774 samples with none lost.
`put_fixed_block` falls from 4.61% to 1.05% self cycles; allocator/free
symbols disappear from the report. The remaining fixed-block work is
necessary emission for winning shards. The exact cost model remains at
2.12%, so it is now the only universal block-candidate work.

The exact EXP-0130 binary has SHA-256
`4bf7366047a4259375b154503f53f642e5e1649f2c86dc6c9b70f783be5b4dd9`;
the fixed candidate has SHA-256
`1c493be6131e8752ee55e9c32949e7c2ef9c6a9d6a4a4505d4bd223e900fc072`.

Artifacts:

- `artifacts/exp0133-lazy-block-confirm.tsv`
  (`a146ecb9a180f8e88282dc6035d628f62c8d832c1ca6aaa965d00c02be413c76`);
- `artifacts/exp0133-stage-perf-stat.tsv`
  (`f966fd1db37afae8d29b242cee60a1aa3abb2ec858de700a1008b979eab3d0d0`);
- `artifacts/exp0133-stage-perf.data`
  (`d9439f852a507321e50cb4af73abc35ae8dba2b3e44387028d676ed74bc9d28c`);
- `artifacts/exp0133-stage-perf-report.txt`
  (`64e473741c7df80ab7332dbe8697cbc875694c4f8c5e42cb67d69537960cebcc`).

All 69 release library tests and nine binary targets pass. Strict debug and
release Clippy with all features, formatting, and diff checks pass.

## Decision

Accept lazy fixed-block emission. It is exact, broadly faster, and removes an
allocation and complete losing-body write from the shard hot path without
changing the low-serialization format.

Relative to EXP-0110's fixed version-2 rows, version-5 geometric encode
advances from about 0.694x to about 0.769x. It remains non-promoted, but the
gap has narrowed to approximately 23.1%. Further CPU exploitation should
target the universal Rice selector or exact fixed-block cost, while format
exploration should remain focused on bounded GPU codewords and
scan/disjoint-write output.

## References

- [Research 0039](../research/0039-parallel-rice-bitstream-hardware.md)
- [EXP-0120](EXP-0120-direct-rice-lane-emission.md)
- [EXP-0124](EXP-0124-post-direct-emission-profile.md)
- [EXP-0130](EXP-0130-four-parameter-rice-pass.md)
