# EXP-0122 — Isolated decode harness

Status: **ACCEPTED**

## Classification

**Evaluation infrastructure** — implement the isolated-phase protocol required
by EXP-0121 before reconsidering byte-identical encoder-only changes.

## Hypothesis

A `fastvid` decode-only benchmark subcommand can retain the complete combined
binary's code layout while excluding encoding, source conversion, file I/O,
and quality metrics from its timed region. Repeated warm-cache decode of one
validated encoded frame should provide a fast, balanced diagnostic for
source-identical decoder movements.

## Modification

Add `benchmark-decode16 INPUT THREADS REPETITIONS` to the existing `fastvid`
binary. Read and decode once before timing, then repeatedly call the normal
`decode16` API and retain outputs through `black_box`. Report encoded bytes,
bit depth, repetitions, luma pixels, decode milliseconds, luma MP/s, and raw
decimal MB/s.

Do not add a new codec path or change encode/decode behavior.

## Test

- reject zero threads and repetitions through existing/new validation;
- verify a known version-5 control decodes and reports the expected bit depth,
  dimensions, encoded size, and positive throughput;
- ensure file I/O and warm-up are outside the timed region;
- pass the full release suite, strict Clippy, formatting, and diff checks;
- use separately preserved complete `fastvid` binaries and alternating order
  for subsequent phase-isolation comparisons.

## Result

The new command reads and warms the accepted version-5 HDR q90 control before
starting its timer. A 20-repeat one-thread diagnostic reports:

```text
encoded_bytes=1735875
bit_depth=10
luma_pixels=2073600
decode_ms=682.613
decode_mpps=60.755
decode_raw_mb_s=243.019
```

The encoded size, bit depth, and 1920x1080 luma area match the accepted
control. High-bit YUV 4:2:2 stores four raw bytes per luma pixel, so the
reported raw throughput is exactly four times luma MP/s apart from decimal
rounding. Zero repetitions are rejected before the timed loop; zero threads
are rejected by the warmed normal decoder.

The release binary containing the harness has SHA-256
`a1ccd598dabfdb20b86ac1752d7c2ba8f7961e66bc9435f8a47c2ca8fb3eb441`.
The input retains the accepted stream SHA-256
`9a3cf708ecdc73f9f8c15a545b41f761ad1ed844c2b8cb4db42118ce587fce37`.

All 67 library tests, motion/squeeze tests, binary and documentation tests,
normal and profiling-feature strict Clippy, formatting, and diff checks pass.

## Decision

Accept the isolated decode entry point as evaluation infrastructure. It uses
the production decoder in the complete `fastvid` binary and keeps file I/O,
warm-up, and output inspection outside the timed region.

The next experiment should add a balanced arbitrary-binary wrapper, rebuild
the EXP-0117 and direct-writer sources with this identical command surface,
and compare isolated decode before deciding whether the 10.07% direct-emission
encode gain is a viable frontier branch.

## References

- [EXP-0114](EXP-0114-parallel-rice-grouped-emission.md)
- [EXP-0120](EXP-0120-direct-rice-lane-emission.md)
- [EXP-0121](EXP-0121-emission-binary-frontend-counters.md)
