# EXP-0055 — Modeled rANS selector

Status: **ACCEPTED**

## Classification

**Exploitation of the EXP-0054 entropy candidate** — remove redundant exact
rANS simulations while preserving the version-3 format and nearly all modeled
space savings.

## Hypothesis

Choosing the table log from the EXP-0053 normalized logarithmic cost and
materializing exact rANS only for the selected predictor will improve encode
throughput at least 2x relative to EXP-0054 while keeping complete bytes
within 1% of the exhaustive candidate.

## Modification

- Normalize and score table logs 8 through 12 from histograms only.
- During predictor selection, use the modeled complete rANS bytes rather than
  simulating state renormalization for every table log and predictor.
- Build one exact rANS payload for the selected predictor and retain it only
  when its materialized size is strictly smaller than Rice/zero-run.
- Keep decoder, table syntax, normalization, reconstruction, and legacy
  compatibility unchanged.

## Test

- On synthetic histograms, compare modeled and exact sizes for every table
  log and record the selected-log mismatch rate.
- Require byte-identical reconstruction and exact q100 output.
- Run the six-trial fast-feedback A/B against both practical v2 and preserved
  EXP-0054.

## Gate

- At least 2x encode-throughput improvement over EXP-0054.
- Complete bytes no more than 1% larger than EXP-0054 in any fast case.
- Decode throughput unchanged within 5% relative to EXP-0054.
- If promising, advance to the complete 8-bit matrix and profile remaining
  encoder/decoder costs.

## References

- [EXP-0053](EXP-0053-finite-block-order0-model.md)
- [EXP-0054](EXP-0054-8bit-tile-rans-format.md)

## Result

The normalized logarithmic selector exactly matched the exhaustive
materialized table-log choice in 256 deterministic synthetic distributions:
zero log mismatches and zero payload-byte overhead. The broader unit suite
also bounds any selected predictor payload to eight bytes above the exact
five-predictor oracle (twice the measured four-byte per-candidate model-error
bound).

Against EXP-0054, modeled selection retained byte-identical streams in all
four fast-feedback cases while reducing candidate encode time by 5.4x to
5.7x. The final source-state binary, compared with the practical version-2
frontier over six balanced trials, measured:

| Case | Bytes | Encode time | Decode time |
|---|---:|---:|---:|
| camera 1080p q90 | -5.24% | +16.47% | +36.52% |
| hard cuts 1080p q90 GOP 12 | -14.31% | +24.16% | +29.51% |
| synthetic grid 4K q100 | -44.14% | +22.59% | +53.17% |
| UI motion 720p q90 GOP 12 | -37.29% | +19.48% | +40.76% |

The focused two-trial, one-thread 18-sample corpus confirmation measured:

- stills q90/q100 combined: 14.97% fewer bytes, 19.14% encode-time
  geometric-mean cost, and 47.29% decode-time cost;
- intra video q90/q100 combined: 8.75% fewer bytes, 16.73% encode-time cost,
  and 48.10% decode-time cost;
- GOP-12 video q90/q100 combined: 11.61% fewer bytes, 14.81% encode-time
  cost, and 55.27% decode-time cost.

No focused cell expanded relative to version 2. The only zero saving was the
q90 procedural chroma-edge still, where the exact fallback retained the
legacy entropy payload. The current complete intra corpus reaches 9.203x
geometric-mean compression at q90 and 5.990x at q100, with q100 exactness
unchanged.

Warm-cache q90/GOP-12 single-frame access across all six clips and standard
target indices read 22.15% fewer encoded bytes by geometric mean. Scalar rANS
decoding increased access latency by 57.80%; the latency cost grew from
47.51% at keyframes to 67.75% with eleven dependency frames. This is an
explicit maximum-compression tradeoff, not a balanced-line result.

The exact-stream control is the q100 640x360 resolution grid:

- encoded bytes: 46,971;
- stream SHA-256:
  `121b1b16cb432bb7a8df438cda076dd6d8f1bb3159de3488f0202b626766764f`;
- decoded/source SHA-256:
  `7d6271074a1894d84bf9135a5cf3a4e5b43e61a5daeee1ad307fd76ac883144b`.

Source and preserved binary:

- implementation source commit: `84a3be1`;
- `artifacts/frontier/fastvid-rans-exp0055`:
  `dda826459cfa9cb017b751749d2b780419b18cc1a2ff9ff309492ea8b4df61da`.

Benchmark artifacts:

- final fast feedback:
  `artifacts/exp0055-fast-feedback-final.tsv`
  (`72cf70a689f7a41d85941c37b0443cdb439545883fecf41abc66f99dcf960df7`);
- focused stills:
  `artifacts/exp0055-images-q90-q100-t1.tsv`
  (`f4287b90b3f946750c3b92ddd63a13d60af9982e38e35df75fdf854d1d8d54e8`);
- focused intra video:
  `artifacts/exp0055-videos-intra-q90-q100-t1.tsv`
  (`14ace1f19b33f615c388c9e9e74d242925b0640196e25d8f59124b68f894a3e7`);
- focused GOP-12 video:
  `artifacts/exp0055-videos-gop12-q90-q100-t1.tsv`
  (`f65eebc45479aee4b607b398fcd473631edf6d71ac833f830baf6417d2169e4`);
- focused access:
  `artifacts/exp0055-access-q90-gop12.tsv`
  (`ea8486df56a5373d42ccf64d77d533b2975ad933e80c2fbf672b441ed90f2d36`);
- maximum-compression comparison:
  `artifacts/exp0055-vs-max-fast-feedback.tsv`
  (`e1c2aaaf580f94ac793ddba6fca67a28a9ff68a7d0bfb25b3b17731877f0d9ac`).

The focused matrices were produced before the final relink caused by adding
the test-only synthetic-distribution check. The final binary retained the
same exact-stream control and reproduced every fast-case byte count; its
six-trial fast timing above is the source-state performance control.

## Decision

Accept as the 8-bit maximum-compression frontier. The implementation clears
the EXP-0054 speed gate by more than 5x, gives back no measured bytes, and
delivers broad 5% to 44% fast-case savings plus 9% to 15% focused aggregate
savings. Preserve version 2 as the practical compression line because
version 3's scalar decode and access costs are substantial. Optimize the rANS
decoder before considering version 3 for the balanced/default line.
