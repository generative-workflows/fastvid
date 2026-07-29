# Corrected Butteraugli frontier

Date: 2026-07-29

## Scope

This corrects metric-dependent conclusions in
[0047](0047-routes-to-ten-x-intra-compression.md) after `dc8d2e5` changed the
evaluator from one-nit Butteraugli to libjxl-compatible 80-nit infinity norm.
Pre-correction scores are not comparable.

## Canonical measurements

| Quality | Tier | Ratio | Min SSIMULACRA2 | Max Butteraugli | Result |
|---:|---|---:|---:|---:|---|
| 90 | rejection | 6.188001x | 93.697319 | 0.803438 | pass |
| 80 | rejection | 7.880526x | 81.434586 | 2.026228 | fail |
| 70 | rejection | 9.284468x | 68.092438 | 3.002337 | fail |
| 90 | full | 6.378451x | 73.365837 | 4.157670 | fail |

Artifacts: `/tmp/fastvid-fixed-butteraugli-baseline-rejection.json`,
`/tmp/fastvid-fixed-butteraugli-q80-rejection.json`,
`/tmp/fastvid-fixed-butteraugli-q70-rejection.json`, and
`/tmp/fastvid-fixed-butteraugli-baseline-full.json`.

The q90 full baseline fails 47/397 samples: YUV422-8 (26), gray-10 (5),
gray-16 (6), RGB444-10 (2), and RGB444-16 (8). Maximum Butteraugli is 4.157670
on `game-minetest-yuv422-8`; the codec is not a valid full-corpus baseline.

## Consequences

1. Global q80/q70 is ruled out: both metrics fail and Butteraugli is not loose.
2. EXP-0174 confirms 11.35% real byte savings, but aggressive RGB10 produces
   a local Butteraugli peak of 1.123859.
3. YUV422-8 dominates repair. Lossless coding is credible, but its cost must
   be funded by safe cells.
4. Neural and transform 10x routes now require explicit rate control against
   both SSIMULACRA2 and local Butteraugli peaks.

## Next hypothesis

Signal a small per-tile source-activity quantizer class. Refine tiles prone to
local peaks while using coarser RGB10 on masked tiles. It must pass corrected
rejection, repair all five failing full cells, and improve total bytes.

Related: [EXP-0174](../experiments/EXP-0174-corrected-butteraugli-cell-quantizer.md).
