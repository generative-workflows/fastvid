# EXP-0018 — Exact entropy payload allocation

Status: **REJECTED**

## Hypothesis

Deferred zero-run materialization improves encoding broadly because it replaces
one-byte-per-sample speculative capacity with exact payload capacity, reducing
allocation and memory footprint for both zero-run and Rice tiles.

## Modification

Retest the EXP-0017 implementation unchanged, interpreting it as an allocation
layout change rather than a Rice-only write-elision change.

## Test

1. Measure fresh baseline and candidate matrices across all 12 corpus-v2 stills
   at q90/q100, one/four threads, and six balanced alternating timed trials
   after warm-up.
2. Compare per-sample medians and quality/thread geometric means.
3. Require identical encoded bytes and reconstruction metrics.
4. Retain the candidate only if the focused confirmation passes.

## Acceptance criteria

- Every quality/thread encode geomean improves by at least 5%.
- No individual sample/settings encode median regresses by more than 2%.
- Decode geomeans stay within 2%.
- Encoded bytes, PSNR, SSIM, and maximum error remain identical.

## Results

Balanced six-trial changes across all 12 stills:

| Quality | Threads | Encode geomean | Decode geomean | Worst encode row |
|---:|---:|---:|---:|---:|
| 90 | 1 | +11.43% | -2.55% | +5.17% |
| 90 | 4 | +12.36% | -0.36% | +6.14% |
| 100 | 1 | +13.45% | +2.46% | +1.11% |
| 100 | 4 | +11.16% | +1.28% | +3.19% |

Every encode row improved and all encoded bytes and quality metrics matched.
The unchanged q90 one-thread decode path moved -2.55%, narrowly exceeding this
experiment's 2% negative-control bound.

## Conclusion

Rejected under its decode-noise gate despite a strong encode result. A
ten-trial confirmation with the independently validated 3% A/B noise bound is
[EXP-0021](EXP-0021-entropy-allocation-final.md).


## References

- [Research 0012](../research/0012-simd-cache-profiling.md)
- [EXP-0016](EXP-0016-entropy-mode-instrumentation.md)
- [EXP-0017](EXP-0017-deferred-zero-run-materialization.md)
