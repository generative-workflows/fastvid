# Adaptive MED block predictor

## Source and use constraints

Huang-Chun Hsu, Jian-Jiun Ding, and De-Yan Lu,
[*Improved Low Complexity Predictor for Block-Based Lossless Image
Compression*](https://doi.org/10.3390/engproc2025092038), Engineering
Proceedings 92(1), 38, 2025.

The paper is distributed under
[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). This record and the
Fastvid model use the published mathematical idea with attribution. No source
code was available or copied, and no JPEG-LS bitstream syntax, context model,
or normative implementation is reused.

The paper reports residual entropy rather than complete encoded bytes.
Fastvid therefore treats its results as a hypothesis and independently charges
the current bounded-shard representation.

## Predictor

The baseline median edge detector (MED) is the clamped gradient:

```text
base = clamp(left + above - upper_left, min(left, above), max(left, above))
```

The paper's adaptive MED adds the median of the already available prediction
residuals at the left, above, and upper-left positions only when all three
residual signs agree, then clips the result to the sample range:

```text
correction =
    median(res_left, res_above, res_upper_left)  if their signs are equal
    0                                           otherwise
prediction = clamp(base + correction)
```

At a top or left block boundary the unavailable residuals are zero, so their
median contributes zero and the baseline reduces to direct left or direct
above prediction. In Fastvid's near-lossless model, predictor state uses the
dequantized residual `quantized * step`, which is exactly reproducible by the
decoder; it must never use the unavailable original error.

The sign agreement is essential: it restricts feedback to regions where the
three causal errors consistently indicate under- or over-prediction. The
arithmetic needs addition, subtraction, comparisons, sign tests, and a median
of three, but no multiplication or division. It adds a second causal state
row, so it is not a SIMD solution inside one block. Its benefit is potentially
better residual concentration at low scalar cost.

## Reported evidence

Across five image categories, the paper reports average residual-entropy
reductions over MED of:

- 2.26% for whole images;
- 2.70% for 32x32 blocks;
- 2.81% for 16x16 blocks; and
- 2.89% for 8x8 blocks.

It reports that 32x32 adaptive blocks are close to the full-image result and
emphasizes their lack of inter-block dependencies. The tested data are
8-bit still images, and entropy estimates omit Fastvid's per-shard mode,
length, Rice-lane, padding, and fixed-block costs. Neither the percentages nor
the preferred block size can be transferred directly.

## Fastvid model

The first experiment uses fixed 16x16, 32x32, and 64x64 predictor blocks
inside the existing 256x128 access tiles. Every block resets reconstructed
neighbors and adaptive residual state. Block results are scattered into the
tile's canonical raster residual array.

Predictor blocks do not become entropy blocks. The raster residual array is
still split into current 4,096-symbol version-5 shards, with exact selection
among:

- zero-run;
- four-lane Rice, including three lane-length words; and
- 128-symbol fixed block pack.

Every shard mode and `u16` body length is charged. This avoids the repeated
per-band entropy headers that caused EXP-0103's 16-row regression and keeps
the two CUDA scheduling dimensions explicit:

- maximum predictor span: at most `block_width * block_height`;
- maximum entropy span: 4,096 symbols, or approximately one quarter of that
  for selected four-lane Rice.

No block-size default may be selected from the development corpus alone.

## Risks

- Adaptive residual feedback can amplify local errors or alter q90 SSE even
  though the quantizer's pointwise reconstruction bound remains unchanged.
- Smooth synthetic data may prefer the longer full-tile gradient context.
- A 16-bit sparse shard may already choose zero-run at negligible payload
  cost, leaving no rate headroom for a stronger predictor.
- A median correction lengthens the recurrence and may cost more CPU than its
  byte savings justify.
- The paper's fixed-length-coding random-access statement does not apply
  directly to Fastvid's variable-length entropy shards; Fastvid access remains
  at the enclosing indexed tile unless block offsets are separately charged.

## Relevant experiments

- [EXP-0047](../experiments/EXP-0047-compatible-predictor-oracle.md)
- [EXP-0103](../experiments/EXP-0103-independent-predictor-bands.md)
- [EXP-0104](../experiments/EXP-0104-predictor-band-height-ladder.md)
- [EXP-0110](../experiments/EXP-0110-full-tile-bounded-shards.md)
- [EXP-0131](../experiments/EXP-0131-adaptive-med-block-model.md)
