# Chroma-from-luma prediction for independent 4:2:2 tiles

## Question

Fastvid predicts Y, Cb, and Cr independently even though chroma edges often
coincide with reconstructed luma edges. This review asks whether an explicitly
signaled affine chroma-from-luma (CfL) candidate has enough complete-byte
headroom to justify a tile-local format experiment.

## Open sources

- Trudeau, Egge, and Barr, [*Predicting Chroma from Luma in
  AV1*](https://arxiv.org/abs/1711.03951), 2017.
- Alliance for Open Media, [AV1 Bitstream and Decoding Process
  Specification](https://aomediacodec.github.io/av1-spec/).
- Alliance for Open Media, [SVT-AV1 CfL implementation
  appendix](https://gitlab.com/AOMediaCodec/SVT-AV1/-/blob/master/Docs/Appendix-CfL.md).

The paper describes AV1 as royalty-free and reports the openly reproducible
reference-codec experiment. This is sufficient to motivate an independent
model, but not by itself a patent-clearance conclusion for a new Fastvid wire
mode. No AV1 implementation code is copied.

## Reusable structure

AV1 predicts chroma as:

```text
CfL(alpha) = alpha * reconstructed_luma_AC + chroma_DC
```

The decoder first subsamples already reconstructed luma to the chroma
resolution and subtracts its block mean. Zero-mean luma separates the AC
edge/detail contribution from the chroma DC level. The encoder signals alpha;
the DC term comes from an existing chroma predictor. The paper argues that
explicit alpha signaling is cheaper and more accurate than making the decoder
fit a regression from unavailable source chroma.

For 4:2:2, each chroma sample coincides horizontally with two luma samples.
The source combines subsampling and mean removal using fixed-point integer
arithmetic. Its signaled alpha range is zero through two in 1/8 steps with an
independent sign for Cb and Cr.

Reported AV1 results are content-dependent:

- 4.87% average CIEDE2000 BD-rate reduction for still images;
- 2.41% for video;
- larger chroma and screen-content gains; and
- much smaller luma-centric PSNR/SSIM changes.

These numbers are not transferable to Fastvid, whose predictors, residual
coder, tiles, and quality objective differ. They do justify category-aware
measurement including UI/graphics rather than camera-only screening.

## Fastvid model boundary

The initial model should preserve independent tile access:

- reconstructed luma for the same tile is already available because plane
  order is canonical;
- 4:2:2 luma pairs are reduced to one integer AC value per chroma sample;
- a tile-local chroma DC byte and signed 1/8-step alpha are charged explicitly;
- the exact existing zero-run/Rice/rANS selector codes the resulting residual;
- the candidate competes against the exact current chroma tile payload; and
- no format, decoder, or quality behavior changes during modeling.

Signaling a DC byte is deliberately more conservative than AV1's neighboring
DC prediction. It keeps tiles independent and makes the first model free of
cross-tile chroma dependencies. If even this two-byte control model wins,
later work can compare causal tile-internal DC prediction without silently
weakening access.

## Gate

Advance only if complete candidate bytes, including DC and alpha, save at
least 2% of aggregate chroma payload and at least two diverse categories show
positive savings. Report total-stream savings separately because 4:2:2
chroma is only half the raw sample count and tile directories are unchanged.
Reject a camera-only or UI-only win as corpus-specific.

## Relevant experiments

- [EXP-0071: charged chroma-from-luma model](../experiments/EXP-0071-chroma-from-luma-model.md)
