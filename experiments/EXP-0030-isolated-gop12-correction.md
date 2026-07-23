# EXP-0030 — Isolated GOP-12 benchmark correction

Status: **ACCEPTED**

## Hypothesis

The EXP-0027 GOP-12 timing direction will remain positive when rerun without a
second CPU-bound benchmark executing concurrently, but the original numeric
rows must not be used as isolated-host measurements.

## Modification

No codec change. Preserve the same EXP-0026 baseline and EXP-0027 candidate
binaries, then rerun q90/q100, one/four-thread, GOP-12 video trials serially.
This record supersedes only EXP-0027's GOP-12 timing table; its isolated GOP-1
confirmation, exact-stream evidence, and acceptance conclusion remain valid.

## Test

Run six balanced trials per matrix cell with no other benchmark process
active. Require unchanged encoded sizes and quality. Compare the direction
with EXP-0027 while reporting the replacement medians independently.

## Results

| Quality | Threads | Baseline encode | Candidate encode | Change | Decode change |
|---:|---:|---:|---:|---:|---:|
| 90 | 1 | 59.411 MP/s | 67.826 MP/s | +14.16% | +1.79% |
| 90 | 4 | 134.326 MP/s | 139.815 MP/s | +4.09% | -6.04% |
| 100 | 1 | 40.924 MP/s | 53.334 MP/s | +30.32% | +2.25% |
| 100 | 4 | 103.925 MP/s | 123.178 MP/s | +18.53% | +0.28% |

Encode geomean improved **16.40%**, versus the contaminated run's 19.51%.
Decode geomean moved **-0.49%**; the isolated 4-thread q90 decoder outlier is
timing noise because candidate and baseline contain identical decoder code
and streams. Encoded sizes and quality signatures were unchanged.

Full replacement rows are in
`artifacts/exp0027-confirm-gop12-isolated.tsv`.

## Conclusion

Accepted as a measurement correction. EXP-0027's high-bit quantizer table
still decisively passes, but only these isolated GOP-12 numbers should be
quoted. Future CPU-bound benchmark commands must execute serially even when
independent tool calls are otherwise eligible for parallel dispatch.

## References

- [EXP-0027](EXP-0027-high-bit-quantizer-table.md)
- [Evaluation methodology](../EVALUATION_METHODOLOGY.md)
