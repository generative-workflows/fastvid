# EXP-0106 — Diagonal residual-order model

Status: **REJECTED**

## Classification

**Parallel-format exploration** — residual serialization order for the
accepted predictor wavefront branch.

## Hypothesis

Serializing spatial residuals in `x + y` anti-diagonal order should permit
coalesced production by the predictor wavefront without materially harming
compression. On a representative one-frame 8/10/12/16-bit q90 screen:

- modeled Rice bytes must be identical to raster order for every tile;
- aggregate best-mode bytes must regress by no more than 0.5%;
- no sample may regress by more than 2%; and
- at least one sample should improve by 1% or more, demonstrating that
  changed zero-run or fixed-block grouping can sometimes pay for the format
  choice.

## Modification

Add a read-only entropy model which reorders each non-temporal tile by
anti-diagonal, then evaluates the same complete zero-run, Rice, and (where
supported) 128-symbol block-pack payload models in raster and diagonal order.
No codec stream changes.

The fast-feedback corpus takes the first frame from five representative
8-bit classes (natural camera, AI-generated, noisy camera, UI animation, and
4K synthetic graphics) plus every native high-bit sample.

## Gate

- the predeclared rate conditions pass;
- the diagonal traversal contains every source symbol exactly once;
- release tests, formatting, and strict Clippy pass; and
- the output and summary are retained with checksums.

## Result

The q90 screen covered 2,115 spatial tiles. Rice payload size was identical
in raster and diagonal order for every tile, as required by the
order-independent bit sum. Best-mode results were:

| Sample | Raster bytes | Diagonal bytes | Delta |
|---|---:|---:|---:|
| AI greenhouse | 1,404,788 | 1,404,788 | +0.0000% |
| Camera cholla | 1,669,799 | 1,669,799 | +0.0000% |
| Noisy camera | 1,512,171 | 1,512,171 | +0.0000% |
| High-precision UI 12-bit | 1,174,242 | 1,209,137 | +2.9717% |
| HDR gradient 10-bit | 1,718,886 | 1,786,177 | +3.9148% |
| High-precision motion 10-bit | 764,282 | 794,212 | +3.9161% |
| 4K resolution grid | 2,185,839 | 2,308,036 | +5.5904% |
| High-precision motion 16-bit | 197,012 | 216,747 | +10.0172% |
| UI dashboard scroll | 31,392 | 40,619 | +29.3928% |
| **Aggregate** | **10,658,411** | **10,941,686** | **+2.6578%** |

Natural/AI/noisy 8-bit content selected Rice, so its size was unchanged.
Diagonal order damaged locality for the zero-run UI case and for the
high-bit fixed-block grouping. Aggregate, worst-sample, and improvement
conditions all fail.

Artifact:
`artifacts/exp0106-diagonal-order.tsv`
(`02bc2937018a37bc447ae342af416767c05ecdd2861c4668b16f900f813b7cf5`).

The exact traversal/Rice invariant unit test, Python compilation, shell
syntax validation, release formatting, and strict Clippy pass.

## Decision

Reject diagonal residual order as a general format. It buys no Rice rate
because Rice bit count is permutation-invariant, while disrupting the
spatial grouping exploited by zero-run and fixed-block coding.

Retain raster-order entropy syntax. A future parallel predictor kernel should
either scatter residuals to their raster positions before entropy coding or
use a bounded raster-order staging buffer. The zero-rate wavefront execution
accepted by EXP-0105 remains valid; only its direct-output serialization
variant is rejected.

## References

- [Research 0037](../research/0037-parallel-hardware-friendly-codecs.md)
- [EXP-0102](EXP-0102-four-lane-rice-shard-model.md)
- [EXP-0105](EXP-0105-predictor-wavefront-model.md)
