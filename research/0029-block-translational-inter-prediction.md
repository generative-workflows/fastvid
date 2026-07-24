# Block-translational inter prediction

## Question

Fastvid's only temporal predictor reads the co-located sample from the
preceding reconstructed frame. That is excellent for static UI and poor for
camera motion, pans, scrolling layers, and translated animation. Can a small,
integer-pixel block translation reduce temporal residual entropy enough to
justify motion-vector syntax without turning the codec into a long-GOP
delivery format?

## Open sources

- Han et al.,
  [*A Technical Overview of AV1*](https://doi.org/10.1109/JPROC.2021.3058584),
  Proceedings of the IEEE 109(9), 2021, CC BY 4.0.
- Alliance for Open Media,
  [AV1 specification and reference-code portal](https://aomedia.org/specifications/av1/)
  and
  [AOMedia Patent License 1.0](https://aomedia.org/license/patent-license/).
- Xiph.Org,
  [rav1e](https://github.com/xiph/rav1e), including its motion-estimation
  module, BSD-2-Clause with the repository's patent grant.

The first Fastvid experiment copies no motion-search or bitstream code. It
uses the old, elementary operation of comparing integer-translated reference
blocks and records a rate proxy. Any eventual format design requires a
separate patent and license review.

## Findings

The AV1 overview describes translational motion compensation as a core inter
predictor and notes that practical block matching commonly minimizes SAD or
SSE. AV1 then adds fractional-pixel filters, affine/warped models, compound
references, many block sizes, and contextual vector coding. Those extensions
are deliberately out of scope for a first Fastvid model.

rav1e is useful engineering evidence because it is a current safe-Rust,
BSD-licensed encoder with distinct speed levels. Its public feature set
combines inter frames, rectangular RDO-selected blocks, 4:2:2/high-bit input,
and many speed settings. Its motion-estimation implementation is correspondingly
large, reinforcing that Fastvid should establish the attainable residual
savings before building a production search.

Fastvid has unusually favorable constraints for a small experiment:

- only the immediately preceding reconstructed frame is referenced;
- GOP dependency and single-frame preroll need not change;
- integer-pixel translation needs no interpolation filter;
- a tile can always fall back to current co-located or spatial prediction;
- one luma-selected vector can be shared with its two 4:2:2 chroma regions.

The principal cost is encoder search. A full search for every sample would be
inappropriate for the speed frontier. A two-stage proxy can first compare
sparsely sampled luma SAD for displacements on a coarse grid, then compute an
exact folded-residual Rice cost only for the winning vector and the
co-located baseline. Charging two signed vector bytes per block prevents the
model from treating tiny improvements as free.

## Risks and evaluation

- source-frame motion estimates can overstate savings against a reconstructed
  reference below q100;
- block boundaries can expose uncovered regions and discontinuities;
- luma-selected motion may worsen chroma;
- exhaustive search cost may dwarf entropy savings;
- motion-vector tools have substantial patent history despite royalty-free
  modern codecs.

The first gate is therefore intentionally only a potential screen. It must
cover every standard video class, report per-video wins and vector
distributions, and require substantial charged Rice-bit reduction across
multiple natural and synthetic videos. Passing that gate authorizes an exact
reconstructed-reference model, not a format change.

## Relevant experiments

- [EXP-0005: gated temporal prediction](../experiments/EXP-0005-gated-temporal-prediction.md)
- [EXP-0047: compatible predictor oracle](../experiments/EXP-0047-compatible-predictor-oracle.md)
- [EXP-0065: integer block-motion potential](../experiments/EXP-0065-block-motion-potential.md)
