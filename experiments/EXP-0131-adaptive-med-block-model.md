# EXP-0131 — Adaptive MED block model

Status: **REJECTED**

## Classification

**Predictor/space exploration** — test a recent low-complexity predictor as a
separate compression-versus-serialization branch.

## Hypothesis

The 2025 adaptive-MED paper reports 2.70% lower residual entropy than MED at
32x32 blocks. Fixed square blocks also reduce a 32,768-sample full-tile causal
predictor chain to at most 1,024 samples.

After exact current version-5 zero-run/Rice/fixed-block selection, at least
one adaptive 16/32/64 block point should:

- cap the predictor span at 4,096 samples or fewer;
- keep aggregate complete bytes within +1% of full-tile clamp-gradient;
- save at least 1% complete bytes versus the same-size independent MED
  blocks;
- keep aggregate q90 SSE within +1% and every sample within +3%;
- retain exact q100 reconstruction; and
- remain non-dominated in complete bytes, predictor span, entropy span, and
  reconstruction error.

## Modification

Add a read-only native-high-bit model that:

- reconstructs fixed 16x16, 32x32, and 64x64 independent predictor blocks;
- compares current clamp-gradient, ordinary MED, and MED plus the median of
  causal dequantized left/above/upper-left residuals;
- scatters folded residuals back into tile-raster order;
- exactly charges version-5 zero-run, four-lane Rice, fixed-block bodies,
  mode bytes, `u16` lengths, and Rice lane-length words;
- reports complete bytes, SSE, maximum error, maximum predictor span, and
  maximum selected entropy span.

No encoder, decoder, format, default block size, or frontier slot changes.

## Test

- unit-test boundary resets, decoder-available adaptive state, q100 exactness,
  shard control accounting, and pointwise quantizer bounds;
- run q90 and q100 over every checksummed native 10/12/16-bit sample;
- retain per-sample and aggregate rows rather than only entropy estimates;
- apply the declared Pareto and rate/error gates;
- run strict Clippy, formatting, and diff checks.

## Result

The primary-source audit found an essential condition omitted from the first
implementation: adaptive MED applies its median correction only when the
left, above, and upper-left residual signs all agree. After correcting that
condition, the model passes its boundary/reset oracle, reproduces the current
version-5 full-tile payload length exactly, charges three control bytes per
shard, retains the quantizer error bound, and reconstructs every q100 sample
exactly.

Exact q90/q100 aggregate results:

| Block | Complete bytes vs full-tile clamp | Vs block clamp | Vs block MED | Predictor span | Entropy span |
|---:|---:|---:|---:|---:|---:|
| 16x16 | +47.4387% | +23.1321% | +1.3931% | 256 | 4,096 |
| 32x32 | +37.8725% | +27.8138% | +1.2500% | 1,024 | 4,096 |
| 64x64 | +33.7308% | +30.4900% | +1.1875% | 4,096 | 4,096 |

The adaptive correction is slightly worse than ordinary block MED after
actual coding, opposite the paper's entropy-only result. More importantly,
independent square boundaries lose far more against Fastvid's full-tile
clamp-gradient context than adaptive MED recovers.

Per-sample q90 adaptive bytes versus full-tile clamp:

| Sample | 16x16 | 32x32 | 64x64 |
|---|---:|---:|---:|
| HDR gradient 10 | +18.3603% | +12.4656% | +9.9542% |
| Precision motion 10 | +18.4514% | +12.6294% | +10.1280% |
| Precision UI 12 | +25.6151% | +12.9071% | +8.4071% |
| Precision motion 16 | +4.0999% | -14.3869% | -21.4719% |

Sparse 16-bit motion is a real content-specific win, but every general point
fails the aggregate and per-content rate gates. Aggregate q90 SSE changes by
-0.0474%, -0.0427%, and -0.0235%; the worst per-sample increases are only
+0.0059%, +0.0055%, and +0.0075%. Quality is not the obstacle.

The raw artifact is
`artifacts/exp0131-adaptive-block-model.tsv`
(`ebe2324a1e14994b46e5ca04984e42083ba670b1ec67413ea7f1eaa8da55ccce`).
The exact model test, strict release Clippy, formatting, and shell syntax pass.

## Decision

Reject adaptive MED and fixed square predictor blocks as a general version-5
format branch. None passes the complete-byte gate, and adaptive MED itself
does not beat same-size MED under the actual entropy representation.

Do not retain the one-off implementation in the active CPU codebase. The
paper, immutable result, artifact hash, and Git history preserve the evidence.
The 64x64 sparse-16-bit result is insufficient to justify content-conditioned
block geometry before GPU migration; it would add a selector, divergent
predictor kernels, and another format mode fitted to one procedural sample.

## References

- [Research 0041](../research/0041-adaptive-med-block-predictor.md)
- [EXP-0103](EXP-0103-independent-predictor-bands.md)
- [EXP-0104](EXP-0104-predictor-band-height-ladder.md)
- [EXP-0110](EXP-0110-full-tile-bounded-shards.md)
