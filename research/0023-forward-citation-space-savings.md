# Forward-citation review: space-saving prediction and symbol models

## Search trail

This pass starts from Fastvid's prior work on
[PNG/Paeth prediction](0002-png-predictors.md),
[adaptive Rice coding](0005-adaptive-rice-coding.md), and
[block-local prediction](0008-block-local-inter-intra-selection.md). It
searched their forward citation trails and related work for publications from
2016 through 2026. Older sources are included only when a recent source or an
actively maintained, permissively licensed specification makes the technique
directly relevant.

The pass deliberately separates three questions:

1. Does a paper report lower entropy?
2. Does the proposed representation actually use fewer encoded bytes after
   mode and table overhead?
3. Is there an implementation-compatible source and an adequate patent grant?

Only the third category is eligible to ground Fastvid's normative format.
Interesting but uncleared work is retained below as a search result, not as
permission to implement it.

## Implementation-compatible sources

- Hsu, Ding, and Lu, [*Improved Low Complexity Predictor for Block-Based
  Lossless Image Compression*][adaptive-med], 2025, CC BY 4.0.
- Google, [*Specification for WebP Lossless Bitstream*][webp-spec], updated
  2023. Text is CC BY 4.0 and code fragments are Apache-2.0.
- Google, [`libwebp` patent grant][webp-patents] and
  [BSD-licensed implementation][webp-code].
- Bennett, [*Benchmarking Lossless Still Image Codecs: Perspectives on
  Selected Compression Standards From 1992 through 2022*][benchmark], 2023,
  CC BY 4.0.
- Giesen, [*Interleaved Entropy Coders*][interleaved], 2014, public preprint,
  and Collet, [Finite State Entropy][fse], dual BSD/GPL implementation.

[adaptive-med]: https://www.mdpi.com/2673-4591/92/1/38
[webp-spec]: https://developers.google.com/speed/webp/docs/webp_lossless_bitstream_specification
[webp-patents]: https://chromium.googlesource.com/webm/libwebp.git/+/HEAD/PATENTS
[webp-code]: https://chromium.googlesource.com/webm/libwebp.git/
[benchmark]: https://library.imaging.org/archiving/articles/20/1/34
[interleaved]: https://arxiv.org/abs/1402.3392
[fse]: https://github.com/Cyan4973/FiniteStateEntropy

## Predictor-bounded residual mapping

The 2025 improved-MED paper observes that a predictor fixes the feasible
residual interval. For an 8-bit prediction `p`, the unquantized error is in
`[-p, 255-p]`, not the full symmetric `[-255, 255]`. Its proposed mapping uses
that fact to avoid carrying an unnecessary sign alternative after the shorter
side of the interval is exhausted. The paper reports this as a way to save the
sign bit.

This is unusually well matched to Fastvid:

- the decoder already knows the causal prediction and quantization step;
- the mapping is a tile-payload interpretation, so the existing directory
  mode byte can signal it without growing directory entries;
- zero still maps to zero, preserving zero-run behavior;
- the mapped alphabet has at most `max_sample + 1` values rather than
  `2 * max_sample + 1`;
- it applies equally to spatial and previous-frame prediction; and
- it changes neither the quantized reconstruction nor random access.

For near-lossless coding the valid interval must be derived in quantized
units using Fastvid's exact rounding rule. If `lo = quantize(-p, step)` and
`hi = quantize(max_sample-p, step)`, an implementation must provide and prove
a bijection between every integer in `[lo, hi]` and `[0, hi-lo]`. Modeling
must use exact Rice and zero-run bytes, not the paper's residual entropy.

This is the highest-priority space experiment because it can reduce symbol
magnitudes without another prediction pass or side table.

## Low-cost block predictor selection

WebP lossless normatively selects one of fourteen causal predictors per
square block. Its compatible source and patent grant cover simple modes such
as left, above, their average, several neighbor averages, select, and clamped
add-subtract predictors. Predictor metadata is itself compressed.

The 2025 improved-MED paper reports that adding the median of three causal
prediction errors to MED reduced residual entropy by 2.26% for full images
and 2.70--2.89% for 32-, 16-, and 8-pixel blocks. It is multiplication- and
division-free, but it introduces more causal state than Paeth and reports
entropy rather than complete coded bytes.

Fastvid should first run an offline per-tile oracle over a small compatible
set:

1. accepted Paeth;
2. left/above average;
3. clamped `left + above - upper_left`;
4. WebP's half-gradient predictor; and
5. adaptive MED only after its state and IP assumptions receive a separate
   review.

The current 32-byte tile directory already carries a prediction-mode byte, so
these alternatives need no additional directory bytes. Exhaustively encoding
several predictors would multiply encoder work, however. A format experiment
is justified only if an oracle shows meaningful total stream savings; a later
experiment must then find a cheap selector or a restricted content-specific
mode.

## Entropy coders and actual-byte accounting

The 2023 benchmark compares actual codec outputs with source-entropy estimates
and uses the gap to characterize practical encoder efficiency. That
distinction matters here: smaller residual entropy can lose after byte
rounding, per-tile mode costs, probability tables, and escape syntax.

ANS/rANS can approach arithmetic-coder density while retaining table-driven
state transitions, and interleaving can expose instruction-level parallelism.
Finite State Entropy provides compatible code for an isolated model. It is
not the first experiment:

- Fastvid tiles are small enough that normalized frequency tables may dominate;
- the accepted Rice and zero-run modes have no transmitted probability table;
- random tile access forbids amortizing state across unrelated tiles; and
- the project currently avoids C dependencies and unsafe code.

Any ANS follow-up must charge normalized-count tables, state flush bytes,
mode signaling, and tile padding. A Shannon-entropy estimate alone is not an
implementation gate.

## Screened but not eligible as grounding

Žalik et al., [*A Case Study on Entropy-Aware Block-Based Linear Transforms
for Lossless Image Compression*][entropy-aware], 2024, is a useful
forward-citation result: its exhaustive block selector reports smaller
estimated byte counts than whole-image JPEG-LS on 30 of 31 images, with gains
up to 5.6%, and explicitly charges control bits. However, the article is
CC BY-NC-ND 4.0. Fastvid therefore does not use it as an implementation
source. It only strengthens the independent experimental question already
posed by the permissively sourced WebP design.

The 2025 [JPEG XL overview][jxl-overview] and
[JPEG XL whitepaper][jxl-whitepaper] describe a self-correcting weighted
predictor and context selection in Modular mode. The reference repository has
a PATENTS file whose implications have not been cleared for a distinct MIT
codec. These tools are comparison targets only.

Learned lossless predictors and integer-only flows were also screened. Their
model storage, compute cost, weak tile-random-access fit, and often separate
weight/code licensing make them unsuitable for the current intermediate-codec
speed target.

[entropy-aware]: https://www.nature.com/articles/s41598-024-79038-2
[jxl-overview]: https://arxiv.org/abs/2506.05987
[jxl-whitepaper]: https://ds.jpeg.org/whitepapers/jpeg-xl-whitepaper.pdf

## Ordered follow-up

1. Model predictor-bounded mapping with exact current zero-run/Rice selection
   on every standard 8-bit and high-bit corpus tile.
2. Implement it only if aggregate stream savings survive directory bytes and
   no predeclared corpus category regresses materially.
3. Run the compatible predictor oracle after the mapping result is known, so
   candidates are scored under the best symbol representation.
4. Consider an ANS table-overhead model only if Rice remains a material gap to
   measured symbol entropy.
5. Return to SIMD only after these format-neutral and format-level space
   opportunities have been resolved.

## Relevant experiments

- [EXP-0038: byte-oriented residual format
  modeling](../experiments/EXP-0038-byte-oriented-residual-model.md)
- EXP-0046: predictor-bounded residual mapping model (planned)

