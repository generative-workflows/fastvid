# Modern descendants of SSIM and efficient quality evaluation

## Citation trail

This note follows forward citations from Wang et al.'s 2004 SSIM paper,
reviewed in [research 0003](0003-ssim.md). The papers below were published
between 2018 and 2024 and either cite SSIM directly or build on a later paper
in that line.

## Sources

- Venkataramanan et al., [*A Hitchhiker's Guide to Structural
  Similarity*][hitchhiker], IEEE Access, 2021.
- Venkataramanan et al., [*One Transform To Compute Them All: Efficient
  Fusion-Based Full-Reference Video Quality Assessment*][funque-plus], 2023.
- Ding et al., [*Image Quality Assessment: Unifying Structure and Texture
  Similarity*][dists], TPAMI, 2020/2021, and the [MIT-licensed reference
  implementation][dists-code].
- Ding et al., [*Locally Adaptive Structure and Texture Similarity for Image
  Quality Assessment*][adists], ACM Multimedia, 2021.
- Mantiuk et al., [*ColorVideoVDP: A visual difference predictor for image,
  video and display distortions*][cvvdp], SIGGRAPH, 2024, and its
  [MIT-licensed implementation][cvvdp-code].
- Siniukov et al., [*Hacking VMAF and VMAF NEG*][vmaf-hacking], 2021.
- Blau and Michaeli, [*The Perception-Distortion Tradeoff*][tradeoff], CVPR,
  2018.

[hitchhiker]: https://arxiv.org/abs/2101.06354
[funque-plus]: https://arxiv.org/abs/2304.03412
[dists]: https://arxiv.org/abs/2004.07728
[dists-code]: https://github.com/dingkeyan93/DISTS
[adists]: https://arxiv.org/abs/2110.08521
[cvvdp]: https://arxiv.org/abs/2401.11485
[cvvdp-code]: https://github.com/gfxdisp/ColorVideoVDP
[vmaf-hacking]: https://arxiv.org/abs/2107.04510
[tradeoff]: https://openaccess.thecvf.com/content_cvpr_2018/html/Blau_The_Perception-Distortion_Tradeoff_CVPR_2018_paper.html

## Efficient SSIM is a methodology change, not merely an optimization

The 2021 SSIM review documents materially different window, scaling, border,
and pooling choices across public implementations. A metric name alone is
therefore insufficient for reproducibility. It recommends reporting the exact
implementation choices and evaluates five repeated timing runs by their
median.

Two acceleration ideas are especially relevant:

- for rectangular windows, five summed-area tables reduce each local
  mean/variance/covariance query to constant work after an image-sized setup;
- sampling local windows with stride `s` reduces nominal work by `s^2`. On the
  four subjective databases tested by the authors, rank correlation was
  largely unchanged through stride 5, for a nominal 25x reduction in local
  evaluations.

That is evidence for a Fastvid experiment, not permission to replace the
standard score. The paper tested correlation against subjective databases,
not agreement with Fastvid's current non-overlapping 8x8 block score on this
corpus. A strided result must be labeled approximate and validated for
ordering, absolute error, and false acceptance/rejection before use in fast
feedback.

## Shared transforms make a slow video metric less expensive

FUNQUE+ cites both the original SSIM work and the 2021 review. It shares one
perceptually weighted transform among several feature families rather than
recomputing separate transforms. The authors report 3.8--11x computational
efficiency improvement and 4.2--5.3% prediction-accuracy improvement over
their comparison models. The architecture is useful to the evaluation
harness: decode and color conversion should be shared, and expensive features
should be computed once per frame/scale.

The result does not establish FUNQUE+ as a drop-in replacement for VMAF or
SSIM. It supports a slow temporal-quality tier and a general rule against
duplicating transforms among metrics.

## Texture, generated content, and geometric tolerance

DISTS combines feature-map correlations for structure with correlations of
spatially averaged features for texture. Its stated motivation is that
pixelwise metrics penalize plausible resampling of texture too strongly.
A-DISTS extends it by estimating whether local regions are structured or
textured and adapting the pooling weights.

These are useful diagnostics for the corpus's AI-generated and fine-texture
assets, and DISTS's implementation is MIT licensed. They depend on learned
feature extraction and RGB conversion, so neither belongs in the fast tier or
as the sole acceptance metric. They also should not excuse deterministic
geometric errors in UI, text, or chroma-edge assets.

## HDR, color, and temporal artifacts

ColorVideoVDP models luminance and chromatic spatial/temporal sensitivity,
viewing conditions, and display photometry. It handles SDR and HDR and can
produce a scalar, temporal distribution, or spatial heat map. The
implementation is MIT licensed but uses PyTorch and is substantially more
expensive on CPU.

This makes it a strong release/diagnostic metric once native HDR color
metadata is carried end-to-end. Results are only reproducible when the metric
version, display model, transfer function, color space, frame rate, and
temporal padding are recorded.

## Guardrails

The VMAF adversarial-preprocessing study increased metric scores while
subjective quality often stayed unchanged or worsened. The
perception-distortion theorem likewise shows that distortion and perceptual
distribution quality are competing objectives in general. Fastvid should
therefore:

- retain exact error, PSNR, and a specified SSIM as independent anchors;
- use texture- and temporal-aware metrics as additional axes, never as the
  only quality gate;
- preserve per-sample results so synthetic graphics, texture, camera noise,
  HDR, and animation cannot hide one another in a corpus mean;
- avoid tuning codec parameters against a single learned metric.

## Bounded follow-up

1. Measure stride-2 and stride-5 variants of the current block-SSIM diagnostic
   against the exact score on every corpus rate point.
2. Add DISTS only as a slow still-image diagnostic after pinning model weights,
   RGB conversion, and dependency versions.
3. Add ColorVideoVDP only after native HDR metadata is supported; pin its
   display model and report CPU/GPU execution separately.
4. Evaluate a shared decoded-frame/color-conversion cache before combining
   several slow metrics.

## Relevant experiments

- [EXP-0037: sampled block-SSIM fast diagnostic](../experiments/EXP-0037-sampled-block-ssim.md)
