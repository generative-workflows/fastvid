# EXP-0032 — Rolling high-bit reconstruction state

Status: **PENDING**

## Hypothesis

Replacing the full reconstructed high-bit tile with one rolling row for
spatial prediction, and omitting reconstruction entirely for temporal
prediction, will improve high-bit encode throughput without changing encoded
bytes, quality, compression, or decode performance.

## Modification

Split high-bit tile residual generation into spatial and temporal paths.
Spatial prediction retains only one `u16` row plus scalar left/upper-left
state. Temporal prediction directly folds quantized source/reference
differences and performs no unused reconstruction.

## Test

1. Preserve the EXP-0031 Fastvid binary as the baseline.
2. Require identical encoded bytes across 10/12/16-bit, q90/q100, one/four
   threads, and GOP 1/12.
3. Run the balanced high-bit fast matrix first.
4. Confirm on the full native high-bit supplement only if the fast encode
   geomean improves by at least 3%.
5. Run release tests, strict Clippy, formatting, and the 8-bit regression gate.

## Acceptance criteria

- Every baseline/candidate stream is byte-identical.
- High-bit encode geomean improves by at least 3%.
- No unexplained encode regression exceeds 3%.
- Decode stays within the 5% timing-noise gate.

## References

- [Research 0016](../research/0016-rolling-reconstruction-state.md)
- [EXP-0031](EXP-0031-openapv-matched-baseline.md)
- [Evaluation methodology](../EVALUATION_METHODOLOGY.md)

