# Sampling profile and high-bit quantization layout

## Sources

- Samply project documentation, [command-line sampling profiler][samply].
- Rust standard library, [`Vec<T>` memory layout][vec].

[samply]: https://github.com/mstange/samply
[vec]: https://doc.rust-lang.org/stable/std/vec/struct.Vec.html

Both sources are compatible with Fastvid's implementation policy: Samply is
dual MIT/Apache-2.0 and is used only as a development tool; the Rust standard
library is already the implementation substrate.

## Method

Build the release executable with line/debug information but unchanged
optimization:

```sh
CARGO_PROFILE_RELEASE_DEBUG=1 cargo build --release --bin fastvid
```

Record 1000 Hz on-CPU profiles with Samply's `--save-only` and
`--unstable-presymbolicate` options. The 8-bit case is the 24-frame noisy
camera sequence at q90, one thread, GOP 1. The high-bit case is the 24-frame
16-bit precision-motion sequence under the same settings.

Samply records Linux perf-event stack samples. Counts are statistical
attribution, not cycle-accurate instruction counts and not cache-miss
measurements. Profiles include metric computation because the benchmark
process performs it after the separately timed codec regions; codec functions
are therefore compared by symbol rather than as a percentage of total process
samples.

## Findings

The 8-bit profile attributed leaf samples as follows:

| Symbol | Samples |
|---|---:|
| encode tile closure/residual generation | 1,140 |
| `ResidualAccumulator::finish` | 733 |
| decode `reconstruct_sample` | 603 |
| decode tile/entropy dispatch | 302 |

The 16-bit profile attributed:

| Symbol | Samples |
|---|---:|
| encode tile closure, including inlined entropy work | 622 |
| decode `reconstruct` | 263 |
| decode tile/entropy dispatch | 59 |

The 8-bit encoder already replaces per-sample quantizer division with a
511-entry table. The newly added high-bit encoder still executes `quantize`
with a signed division for every sample. Its complete residual domain has
`2^(b+1)-1` entries. A contiguous `Vec<i32>` therefore costs:

| Bit depth | Entries | Bytes |
|---:|---:|---:|
| 10 | 2,047 | 8,188 |
| 12 | 8,191 | 32,764 |
| 16 | 131,071 | 524,284 |

The 10/12-bit tables are L1-sized on the test host; the 16-bit table is
L2-sized. Rust guarantees `Vec` elements are contiguous, and an immutable
table can be shared across scoped tile workers without copying. Construction
per frame performs at most 131,071 divisions instead of roughly one division
per coded sample. The tradeoff is a data-dependent lookup working set, so
wall-time A/B measurement is required; this profile alone does not prove a
cache benefit.

## Post-table profile

After accepting the exact-domain table, the same 16-bit profile attributed 535
leaf samples to the encode tile closure, down from 622, while the separately
timed benchmark improved from 34.74 to 39.39 MP/s in those sampling runs. The
remaining high-bit entropy selector reads every folded-residual tile once for
each of 17 Rice parameters. A follow-up can compute all 17 quotient sums in
one traversal. This does not reduce shift/add arithmetic, but it reduces
folded-vector reads from 17 passes to one and is therefore a direct
memory-traffic experiment rather than a SIMD claim.

EXP-0028 tested that traversal inversion and rejected it after a 1.94% encode
geomean regression. The result indicates that the existing parameter-outer
form's compiler specialization outweighs the expected reduction in memory
loads on this host; no cache-miss claim can be made without counters.

EXP-0029 retained that loop form but stopped after the first parameter whose
quotient sum was zero. Larger parameters then have the same zero quotient sum
and strictly larger fixed cost. This exact early termination improved the
12-cell GOP-1 encode geomean by 4.32% and the 16-bit GOP-12 geomean by 11.09%.

## Relevant experiments

- [EXP-0022](../experiments/EXP-0022-llvm-vectorization-audit.md)
- [EXP-0023](../experiments/EXP-0023-quantizer-lookup-table.md)
- [EXP-0027](../experiments/EXP-0027-high-bit-quantizer-table.md)
- [EXP-0028](../experiments/EXP-0028-single-pass-high-bit-rice-cost.md)
