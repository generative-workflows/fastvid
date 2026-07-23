# EXP-0037 — Sampled block-SSIM fast diagnostic

Status: **ACCEPTED** (block stride 2 only; block stride 5 rejected)

## Hypothesis

Evaluating every second or fifth 8x8 luma block in each axis will preserve the
ordering and closely approximate Fastvid's complete non-overlapping block-SSIM
score while reducing metric work enough to improve fast-feedback latency.

## Modification

Add explicitly named sampled block-SSIM functions for 8-bit and high-bit
planes. A block stride of one is the current exact score; strides two and five
sample the top-left-anchored block lattice in both axes.

Add a separate diagnostic command that encodes/decodes an input sequence once,
then reports exact, stride-2, and stride-5 scores and their isolated metric
times. Do not change the standard benchmark columns or acceptance score.

This is not the same window geometry as the overlapping-window stride studied
by Venkataramanan et al. Fastvid already evaluates disjoint 8x8 blocks, so the
paper motivates the question but does not predict the result.

## Test

1. Unit-test exact equivalence between the current function and block stride
   one, unit score for identical planes, invalid stride rejection, clipped
   edge blocks, and 8/10/12/16-bit validation.
2. Run qualities 60, 75, 90, 95, and 100 across every standard still and video
   sample, with video sequences evaluated frame by frame.
3. Preserve per-sample/quality rows containing the three scores, absolute
   errors, ordering, metric time, and evaluated-block counts.
4. Report maximum and percentile absolute error, Spearman rank correlation,
   pairwise ordering disagreements, and measured metric speedup.
5. Run release tests, strict Clippy, formatting, and Lean if the diagnostic is
   retained.

## Acceptance criteria

- Block stride one is bit-for-bit the current `f64` result.
- Stride 2: maximum absolute error at most 0.0005, Spearman correlation at
  least 0.999, and no pairwise operating-point reversal larger than 0.0005.
- Stride 5: maximum absolute error at most 0.001, Spearman correlation at
  least 0.995, and no pairwise operating-point reversal larger than 0.001.
- Median isolated metric speedup is at least 2x for stride 2 and 8x for stride
  5 on 1080p-or-larger inputs.
- Approximate scores remain labeled diagnostic and never replace the exact
  release/acceptance score.

## Results

The complete matrix contained 90 rows: all 18 codec-track samples at qualities
60, 75, 90, 95, and 100. Videos used all 24 frames with GOP 12; stills used
GOP 1. Metrics were evaluated three times per stride with rotating execution
order to reduce cold-cache bias. The saved artifact is
`artifacts/exp0037-ssim-sampling.tsv` (SHA-256
`3257b59beb4d489585b1e7c8fb3ff71e5b4c644559603519bb4c261aa6269c38`).

Command:

```text
scripts/benchmark-ssim-sampling.sh
```

Host: 4-vCPU AMD EPYC-Genoa KVM guest, Linux 7.0.0-22-generic, rustc
1.97.1. The release diagnostic binary SHA-256 was
`141f054b8fc84528dac5eba20727dd47a86afbf4751d99154cc8c1ddf833b7d2`.

| Diagnostic | Maximum absolute error | Median error | p95 error | Spearman rho | Reversals | Median >=1080p speedup | Gate |
|---|---:|---:|---:|---:|---:|---:|---|
| block stride 2 | 0.000415405 | 0.000021591 | 0.000192237 | 0.999900428 | 0 | 3.443x | Pass |
| block stride 5 | 0.001987673 | 0.000059320 | 0.001066339 | 0.998672375 | 0 | 19.596x | Fail |

Stride 2's worst row was `bbb-grass-fur` q60: exact 0.951727874,
sampled 0.952143279. Stride 5's worst row was
`noisy-camera-fourpeople` q60: exact 0.940307579, sampled 0.942295251.
The latter exceeds both its maximum-error gate and its p95 error target,
showing that sparse spatial sampling is biased by heterogeneous/noisy camera
content even though global ranking remains strong.

Block stride one returned exactly the same `f64` result as the existing API.
All 27 release tests, strict Clippy, formatting, and the Lean build passed.
The first Lean invocation from the repository root failed because the project
file lives under `specs/`; rerunning `lake build` from `specs/` succeeded.

## Conclusion

Accept block stride 2 as a provisional fast diagnostic. It evaluates about a
quarter of the current 8x8 block lattice and achieved 3.44x measured metric
speedup without a meaningful ordering failure on this corpus. It does not
replace exact SSIM: any candidate advancing beyond fast screening must be
rescored with block stride one.

Reject block stride 5 for standard use. Its speed is attractive, but its
nearly 0.002 worst-case bias is too large for high-quality codec comparisons.
The generic API remains useful for experiments, while the evaluation
methodology permits only stride 2 in fast screening.

## References

- [Research 0018](../research/0018-modern-perceptual-metrics.md)
- [Evaluation methodology](../EVALUATION_METHODOLOGY.md)
