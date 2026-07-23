# EXP-0034 — Perf, Samply, and Cachegrind profile

Status: **ACCEPTED**

## Hypothesis

Symbolized time sampling plus sanity-checked PMU and simulated-cache events can
identify the next high-bit optimization target more reliably than source
inspection alone and can separate CPU-time attribution from cache behavior.

## Modification

No codec modification. Build the accepted EXP-0032 code with release
optimization and debug information in a separate target directory. Profile
the native 10-bit q90 motion case at one thread/GOP 1 with:

- Samply 0.13.1 at 1000 Hz for symbolized on-CPU stacks;
- Linux perf 7.0.12 for cycles, instructions, branches, L1D, and dTLB events;
- Valgrind Cachegrind 3.26.0 for deterministic simulated cache/branch counts on
  a reduced representative case.

## Test

1. Record tool versions, kernel perf policy, input, and exact commands.
2. Reject PMU aliases that return implausible zero counts.
3. Keep codec-internal timing distinct from whole-process counter totals.
4. Preserve raw profiles and summarized hot symbols/events.
5. Select the next optimization only when the sampled hot path and counter
   evidence are compatible.

## Acceptance criteria

- Samply resolves Fastvid codec symbols.
- At least cycles/instructions and one cache hierarchy event pass sanity
  checks.
- Cachegrind completes without codec errors.
- Conclusions distinguish statistical sampling, hardware PMU counts, and
  cache simulation.

## Results

Environment:

- Samply 0.13.1, perf 7.0.12, Valgrind/Cachegrind 3.26.0;
- `perf_event_paranoid=1`, `kptr_restrict=1`;
- release optimization plus debug information in
  `/tmp/fastvid-profile-target`;
- AMD EPYC-Genoa VM, one thread, native 10-bit q90 motion, GOP 1.

The 999 Hz perf time profile captured 1,308 samples with no losses. Largest
exclusive symbols:

| Symbol | Samples |
|---|---:|
| `codec16::finish_entropy` | 27.84% |
| high-bit encode tile closure | 22.36% |
| `codec16::reconstruct` | 17.40% |
| `codec16::decode_tile_payload` | 12.40% |

Samply recorded the same run at 1000 Hz and preserved its profile plus
presymbolication sidecar. Perf was used for the tabular symbol summary because
its local report consumed the debug information directly.

Five-run hardware PMU means for the complete benchmark process:

- 4.505 billion cycles (±0.29%);
- 16.445 billion instructions (±0.00%), or 3.65 instructions/cycle;
- 2.631 billion branches and 36.17 million branch misses (1.37%);
- 3.617 billion L1D loads and 63.28 million L1D load misses (1.75%).

The generic `cache-misses` alias and a legacy branch-miss alias returned
implausible zeros and are excluded. dTLB aliases produced counts but higher
variation and are retained only in the raw artifact.

Cachegrind on the one-frame 10-bit HDR case simulated a 32 KiB L1 and 32 MiB
last-level cache:

- 330.98 million data references;
- 5.343 million L1D misses (1.6%);
- 586,114 last-level data misses (0.2%);
- 235.50 million branches, 6.272 million simulated mispredicts (2.7%).

The `finish_entropy` aggregate accounted for 2.486 million simulated L1D read
misses, **58.2% of all L1D read misses**, while also leading time samples.
This makes folded-residual entropy processing the next evidence-backed target.
The sampling, PMU, and simulation results are not treated as interchangeable:
they independently agree only on a cache-active, CPU-hot entropy stage.

Artifacts:

- `artifacts/exp0034-samply-10bit-q90.json.gz` and sidecar;
- `artifacts/exp0034-perf-time.data`;
- `artifacts/exp0034-perf-core.txt`;
- `artifacts/exp0034-perf-cache.txt`;
- `artifacts/exp0034-cachegrind.out`;
- `artifacts/exp0034-cachegrind-summary.txt`.

## Conclusion

Accepted as a profiling result. Direct PMU events are available on this host;
the prior statement that hardware counters were unavailable was too broad and
has been corrected. The next optimization should reduce `finish_entropy`'s
folded-residual working set or passes and must be verified with both balanced
wall time and repeated L1D events.

## References

- [Research 0012](../research/0012-simd-cache-profiling.md)
- [Research 0014](../research/0014-sampling-and-high-bit-quantization.md)
- [EXP-0032](EXP-0032-rolling-high-bit-reconstruction.md)
