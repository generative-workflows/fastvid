# Tile geometry as a codec tradeoff

## Question

Fastvid has used 256x128 luma tiles as a fixed default since its first format,
but no experiment has measured that choice. Tile geometry changes several
objectives at once:

- smaller tiles expose more parallel work and reduce tile-level access
  amplification;
- larger tiles amortize directory and entropy-table overhead;
- each independent tile restarts spatial prediction and entropy adaptation;
- cache footprint and worker scheduling depend on both width and height.

Is tile geometry a missing branch of the technology tree rather than a
constant to inherit?

## Open sources

- JPEG XL reference implementation,
  [`doc/encode_effort.md`](https://github.com/libjxl/libjxl/blob/main/doc/encode_effort.md),
  current main branch, BSD-3-Clause plus its published patent grant.
- Lim et al.,
  [RFC 9924: Advanced Professional Video](https://www.rfc-editor.org/rfc/rfc9924.html),
  February 2026.
- Academy Software Foundation,
  [OpenAPV](https://github.com/academysoftwarefoundation/openapv), BSD-3-Clause.

These sources motivate measurement and interoperability constraints only.
Fastvid already has its own rectangular-tile syntax and implementation; no
external codec code is copied.

## Findings

JPEG XL's lossless effort ladder is unusually direct evidence that spatial
grouping is an encoder decision. Its fastest modular tier fixes the predictor,
transform, and entropy choices. Higher tiers add progressively more modeling,
and its expert tier additionally tries different group dimensions. This
places geometry alongside predictor and entropy selection as a compression
search dimension, not merely a threading parameter.

RFC 9924 defines APV frames as raster-ordered rectangular tiles. Tiles are
processed independently, share a nominal width and height except at frame
boundaries, and exist to enable parallel encoding and decoding. APV therefore
also treats independent geometry as part of a professional high-throughput
codec's normative structure.

Fastvid's tradeoff differs from both:

- every plane tile has directory/control overhead;
- version 3 may pay a separate rANS frequency table per tile;
- all spatial predictors restart at tile boundaries;
- the current scheduler exposes one work item per tile;
- individual-tile decoding reads and reconstructs exactly one tile, so area
  is a direct access-amplification bound.

The best width and height need not be equal. Wider tiles preserve horizontal
causal context and reduce column-boundary resets, while shorter tiles retain
more parallel rows and a smaller rolling working set. The existing 256x128
default expresses that idea, but without evidence.

## Proposed Fastvid experiment

First make the benchmark commands report and optionally override rectangular
tile geometry without changing codec defaults. Then screen:

- 64x64;
- 128x64, 128x128, and 192x192;
- 256x64, 256x128, 256x256, 256x384, and 256x512;
- 384x256;
- 512x128, 512x256, and 512x512;
- 1024x128 and 1024x256.

Use complete encoded bytes, encode/decode MP/s and MB/s, stream bitrate,
entropy-mode counts, tile count, and unchanged quality metrics. The first
screen is single-trial and serial. Only geometries that materially improve
rate or speed advance to balanced repeated measurements.

Tile-level access must be charged separately from frame access. For a requested
sample or small region, the nominal decoded area is
`tile_width * tile_height`, clipped at frame edges. A larger geometry is not a
free compression win if it makes localized editing disproportionately
expensive.

## Relevant experiments

- [EXP-0057: automated Pareto frontier](../experiments/EXP-0057-automated-pareto-frontier.md)
- [EXP-0061: three-version frontier](../experiments/EXP-0061-three-version-frontier.md)
- [EXP-0064: rectangular tile-geometry sweep](../experiments/EXP-0064-tile-geometry-sweep.md)
