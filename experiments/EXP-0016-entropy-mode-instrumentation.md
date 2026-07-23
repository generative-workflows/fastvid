# EXP-0016 — Untimed entropy-mode instrumentation

Status: **ACCEPTED**

## Hypothesis

Recording entropy and prediction tile counts outside timed regions will reveal
which corpus classes pay for discarded candidate payloads and will guide
memory-layout optimization without perturbing reported encode/decode time.

## Modification

Extend `StreamInfo` with zero-run/Rice and spatial/temporal tile counts derived
from the existing tile directory. Append aggregate counts to
`benchmark-yuv422` output after encoding has stopped timing.

No bitstream field or timed codec operation changes.

## Test

1. Verify counts sum to total encoded tiles.
2. Run the fast tier and inspect mode distribution.
3. Confirm encoded sizes and quality metrics match the accepted baseline.
4. Run release tests and strict lints.

## Acceptance criteria

- Counts are internally consistent.
- Existing bitstreams and benchmark measurements remain compatible.
- The instrumentation identifies whether the deferred-zero-run experiment
  applies primarily to Rice or zero-run tiles.

## Results

The fast-tier mode totals were deterministic across all five trials:

| Case | Zero-run tiles | Rice tiles | Spatial tiles | Temporal tiles |
|---|---:|---:|---:|---:|
| grid-4k | 765 | 0 | 765 | 0 |
| camera-1080p | 0 | 216 | 216 | 0 |
| ui-temporal-720p | 360 | 0 | 360 | 0 |
| cuts-temporal-1080p | 864 | 0 | 216 | 648 |

Both entropy counts and both prediction counts summed to total tiles in a new
unit test. Encoded sizes matched the accepted baseline. Instrumentation parses
the already-produced directory after encode timing stops and before decode
timing starts.

## Conclusion

Accepted. The detailed camera case is a pure Rice target, while the other fast
cases provide pure zero-run counterweights. This is an unusually clean matrix
for testing deferred payload construction.


## References

- [Research 0012](../research/0012-simd-cache-profiling.md)
- [EXP-0010](EXP-0010-fast-feedback-loop.md)
