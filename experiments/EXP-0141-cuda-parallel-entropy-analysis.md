# EXP-0141 — CUDA parallel entropy analysis

Status: **ACCEPTED**

## Hypothesis

Parallel exact cost reduction within each 4096-symbol shard will remove the
measured 57.842 ms analysis bottleneck while preserving Rust bytes.

## Modification

Replace the serial analyzer with 128 threads per shard. Threads accumulate all
17 four-lane Rice costs, charge each nonzero token, charge maximal zero runs
from run-start threads, and compute independent 128-symbol block maxima.
Shared reductions retain Rust's exact block strict-win and zero-run tie-win
rules.

## Test

Rerun full byte-identity conformance and the same q90/q100 real-world 4K
complete-call benchmark. Profile q90 again.

## Result

All conformance streams remained byte-identical. Q90 improved from 60.366241
to 2.957654 ms, a 20.41x end-to-end speedup, reaching 2.804385 GP/s. Q100
reached 3.049542 GP/s. Analysis fell from 57.842 ms to 389.410 us; prediction
and emission now take 1.039 ms and 1.105 ms respectively.

## Decision

Accept. The q90 path is 6.5% below the >3 GP/s target. Further work should
target prediction/emission and fixed orchestration, not the now-minor analyzer.

## References

- [Research 0042](../research/0042-gpu-variable-output-assembly.md)
- [EXP-0140](EXP-0140-cuda-v5-encoder-baseline.md)
