# EXP-0015 — Temporal zero-copy corpus confirmation

Status: **ACCEPTED**

## Hypothesis

The bulk temporal zero-run reconstruction from
[EXP-0014](EXP-0014-temporal-zero-run-copy.md) is a worthwhile general
optimization even though its largest fast-tier benefit appeared on
scene-cut/grain content rather than the predicted UI case.

## Modification

Retest the EXP-0014 implementation without modification against a freshly
measured baseline across every video in corpus v2.

## Test

Run baseline and candidate matrices over six 24-frame videos at q90 and q100,
one and four threads, GOP 12, and three timed trials after one warm-up. Compare
per-sample median throughput and the geometric mean. Verify encoded sizes and
quality are identical.

## Acceptance criteria

- No quality/thread aggregate decode geomean regresses by more than 2%.
- At least one quality/thread aggregate improves by 5%.
- At least two individual sample/settings improve by 5%.
- Encode geomean remains within 2%; encoded bytes and reconstruction metrics
  are unchanged.

## Results

Corpus-v2 video geomean changes from per-sample three-trial medians:

| Quality | Threads | Encode change | Decode change |
|---:|---:|---:|---:|
| 90 | 1 | +0.53% | **+16.93%** |
| 90 | 4 | +1.53% | **+13.29%** |
| 100 | 1 | +0.89% | **+6.24%** |
| 100 | 4 | +1.98% | **+4.75%** |

No aggregate decode result regressed. Nine of 24 sample/settings improved
decode by at least 5%. The largest gains were:

- q90 one-thread BBB foliage: +53.10%;
- q90 one-thread BBB grass: +30.87%;
- q90 four-thread BBB foliage: +32.49%;
- procedural scene cuts: +18.78% to +20.91% in every setting.

No encoded byte count, Y PSNR, or luma SSIM value changed. The maximum aggregate
encode movement was +1.98%, within the acceptance bound.

Raw and reduced results are in ignored build artifacts:

- `artifacts/temporal-v2-baseline.tsv`
- `artifacts/temporal-v2-zero-copy.tsv`
- `artifacts/temporal-v2-comparison.tsv`

## Conclusion

Accepted. Direct reference-span copying is a substantial safe-Rust temporal
decode optimization. It also demonstrates a practical SIMD strategy: express
bulk contiguous work through optimized standard-library primitives before
adding explicit architecture intrinsics.


## References

- [Research 0012](../research/0012-simd-cache-profiling.md)
- [EXP-0010](EXP-0010-fast-feedback-loop.md)
- [EXP-0014](EXP-0014-temporal-zero-run-copy.md)
