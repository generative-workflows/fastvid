# EXP-0021 — Entropy allocation final confirmation

Status: **ACCEPTED**

## Hypothesis

Exact zero-run payload allocation provides a robust double-digit encode
improvement across the still corpus, while the unchanged decoder remains
within the ±3% noise band established by EXP-0020.

## Modification

Use the unchanged EXP-0017 candidate and balanced A/B harness.

## Test

Repeat all 12 stills at q90/q100 and one/four threads with ten alternating
trials per binary after warm-up.

## Acceptance criteria

- Every encode geomean improves by at least 5%.
- No individual encode median regresses.
- All unchanged decode geomeans remain within 3%.
- Encoded bytes and quality metrics are identical.

## Results

Balanced ten-trial corpus-still changes:

| Quality | Threads | Encode geomean | Decode geomean | Worst encode row |
|---:|---:|---:|---:|---:|
| 90 | 1 | **+13.40%** | +0.62% | +4.74% |
| 90 | 4 | **+12.07%** | -0.17% | +2.31% |
| 100 | 1 | **+11.60%** | +0.06% | +2.09% |
| 100 | 4 | **+10.95%** | -0.28% | +2.07% |

Every individual encode median improved. All encoded byte counts, Y PSNR, and
luma SSIM values matched. The unchanged decode path stayed within ±0.62%.

## Conclusion

Accepted. Deferring zero-run materialization and allocating its winning payload
at exact size is a robust double-digit encode optimization across qualities and
thread counts.


## References

- [EXP-0017](EXP-0017-deferred-zero-run-materialization.md)
- [EXP-0018](EXP-0018-exact-entropy-allocation.md)
- [EXP-0020](EXP-0020-balanced-ab-harness.md)
