# Sampling profilers and contiguous quantization tables

## Sources

- Samply project documentation, [command-line sampling profiler][samply].
- Linux kernel documentation, [`perf_event_open` security][perf-security].
- Rust standard library, [`Vec<T>` guarantees][vec].

[samply]: https://github.com/mstange/samply
[perf-security]: https://docs.kernel.org/admin-guide/perf-security.html
[vec]: https://doc.rust-lang.org/stable/std/vec/struct.Vec.html

## Findings from the sources

Samply records time-based call-stack samples through the platform profiler. On
Linux it uses the perf-events subsystem. Sample attribution is statistical:
it identifies where CPU time is observed, but does not by itself establish
cache-miss counts, instruction counts, or causality. A profiling record should
therefore state sample rate, optimized/debug-info build configuration, input,
and whether non-codec metric work is included.

Linux perf-events permissions are controlled by `perf_event_paranoid` and
capabilities. Access to one's own processes does not imply that every PMU
event is implemented correctly by a virtualized host; counter sanity checks
and repeated measurements remain necessary.

Rust documents `Vec<T>` as a contiguous, heap-allocated sequence with
`capacity >= len`. An immutable vector can be shared across scoped workers.
This makes an exact-domain quantization lookup table a valid safe-Rust layout:
move repeated signed division into table construction, then use contiguous
indexed loads in the sample loop. Table size and cache level remain empirical
tradeoffs rather than consequences of the API guarantee.

Rice parameter search over a fixed residual vector is another profiling
candidate. Reordering loops to reduce apparent memory passes can change
compiler specialization and branch behavior, so traversal-count arguments
must be validated by wall-time A/B and, where available, PMU counters.

## Relevant experiments

- [EXP-0027](../experiments/EXP-0027-high-bit-quantizer-table.md)
- [EXP-0028](../experiments/EXP-0028-single-pass-high-bit-rice-cost.md)
- [EXP-0029](../experiments/EXP-0029-rice-cost-early-termination.md)

