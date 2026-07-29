# Routes to 10x perceptually lossless intra compression

Date: 2026-07-29

## Target and measured frontier

Fastvid stores canonical inputs in 16-bit planar containers. Consequently,
10x does not mean an extreme generative-image rate: it corresponds to about
3.2 coded bits per luma pixel for YUV422, 4.8 bits per pixel for RGB444, and
1.6 bits per pixel for gray before metadata differences.

Canonical rejection-tier measurements on revision `763ef10` locate the current
spatial predictive codec's frontier:

| Quality | Compression | Minimum SSIMULACRA2 | Maximum Butteraugli | Quality failures |
|---:|---:|---:|---:|---:|
| 90 | 6.1880x | 93.6973 | 0.0844 | 0 |
| 80 | 7.8805x | 81.4346 | 0.1750 | 3 |
| 70 | 9.2845x | 68.0924 | 0.2927 | 5 |

Artifacts:

- `/tmp/fastvid-exp0171-neural-entropy-baseline-rejection.json`;
- `/tmp/fastvid-neural-10x-q80-rejection.json`;
- `/tmp/fastvid-10x-frontier-q70-rejection.json`.

Butteraugli remains far inside its 1.0 gate throughout this sweep;
SSIMULACRA2 is the active constraint. The current scalar residual quantizer is
therefore close to 10x in rate but allocates error in a way SSIMULACRA2 strongly
penalizes. A 0.5% entropy improvement cannot bridge the measured gap.

## New primary sources

- The JPEG Committee reports that [JPEG AI became an International
  Standard](https://jpeg.org/items/20250219_press.html) in 2025 with nearly 30%
  improvement over advanced conventional anchors at equivalent subjective
  quality.
- JPEG's [Draft International Standard results](https://jpeg.org/items/20240507_press.html)
  report 12.5--27.9% BD-rate gains over VVC Intra across decoder configurations
  spanning 7--216 kMAC/pixel, with 8/10-bit support and high-to-nearly-lossless
  operating points.
- The official [JPEG AI Call for Proposals result
  report](https://ds.jpeg.org/documents/jpegai/wg1n100250-096-REQ-Report_on_the_JPEG_AI_Call_for_Proposals_Results.pdf)
  reports content-dependent subjective rate reductions of 33% to 65--70% over
  VVC for the strongest learned submissions. This establishes large potential,
  but also unusually high content variance.
- Tatwawadi et al., [What Matters in Practical Learned Image
  Compression](https://openaccess.thecvf.com/content/CVPR2026/html/Tatwawadi_What_Matters_in_Practical_Learned_Image_Compression_CVPR_2026_paper.html),
  CVPR 2026, report 2.3--3x subjective bitrate savings over AV1/AV2/VVC/ECM and
  JPEG AI. Their PICO codec still takes 230 ms to encode and 150 ms to decode a
  12 MP image on an iPhone 17 Pro Max, so its rate result is evidence for the
  perceptual-model opportunity, not a Fastvid-ready runtime architecture.
- Xu et al., [Window-based Channel Attention for Wavelet-enhanced Learned Image
  Compression](https://openaccess.thecvf.com/content/ACCV2024/html/Xu_Window-based_Channel_Attention_for_Wavelet-enhanced_Learned_Image_Compression_ACCV_2024_paper.html),
  ACCV 2024, report 18.5--24.7% BD-rate reductions against VTM-23.1. The useful
  structural evidence is frequency separation plus content-adaptive modeling;
  its attention network is too expensive for Fastvid's gates.
- Song et al., [Variable-Rate Deep Image Compression Through
  Spatially-Adaptive Feature Transform](https://openaccess.thecvf.com/content/ICCV2021/html/Song_Variable-Rate_Deep_Image_Compression_Through_Spatially-Adaptive_Feature_Transform_ICCV_2021_paper.html),
  ICCV 2021, demonstrate pixel-wise quality maps and task-aware bit allocation
  in a single variable-rate model.
- Jia et al., [Bit Distribution Study and Implementation of Spatial Quality Map
  in JPEG AI](https://arxiv.org/abs/2402.17470), 2024, report that spatial bit
  allocation further improves JPEG AI and that VVC's variable block structure
  already provides important adaptive allocation.
- Zhang et al., [LVQAC](https://openaccess.thecvf.com/content/CVPR2023/html/Zhang_LVQAC_Lattice_Vector_Quantization_Coupled_With_Spatially_Adaptive_Companding_for_CVPR_2023_paper.html),
  CVPR 2023, show that lattice vector quantization plus spatially adaptive
  companding improves learned codecs without a large complexity increase.
- Ding et al., [JND-Based Perceptual Optimization for Learned Image
  Compression](https://arxiv.org/abs/2302.13092), 2023, demonstrate that
  distortion-aware JND weighting improves perceptual quality at fixed rate.

## Ranked opportunity areas

### 1. Lossy block transform plus perceptual coefficient quantization

**Potential: large. Runtime fit: strongest. Priority: first.**

Fastvid currently quantizes spatial prediction residuals sample by sample. It
has tested a reversible Haar-like squeeze only at q100; EXP-0075 rejected that
lossless transform at 0.801% savings. That result does not test lossy frequency
transform coding. Quantizing decorrelated DCT or wavelet coefficients by
frequency is the central mechanism that allows energy to be removed where the
visual system is less sensitive while retaining edges and low frequencies.

A tile-independent 8x8 integer transform exposes thousands of independent CUDA
blocks, bounded shared memory, fixed work, parallel inverse transforms, and
simple coefficient scans. It is much more compatible with Fastvid's latency
gates than a hyperprior network. Required safeguards are:

- current spatial mode remains an exact fallback per block/tile;
- q100 selects the current exact path unless an exactly reversible transform is
  separately proven;
- luma/chroma/RGB/gray use depth-scaled quantization matrices;
- DC prediction stays local to an independently decodable tile;
- entropy syntax charges coefficient significance, levels, mode maps, and all
  block metadata;
- no deblocking dependency crosses tile boundaries.

The initial experiment should use one fixed 8x8 transform and one fixed matrix,
not a transform-size search. It is attributable and establishes whether
frequency-selective quantization can improve both rate and SSIMULACRA2.

### 2. Spatially adaptive perceptual quantization

**Potential: medium to large. Runtime fit: strong with simple features.**

The q80 result reaches 7.88x and fails only three rejection samples. This makes
content/activity-adaptive step selection a credible near-term branch. A local
masking model can spend bits on flat gradients, text, and structured edges while
coarsening textured regions. The decoder needs only signaled step classes; the
encoder may calculate a more sophisticated map.

This becomes a plausible route to 10x when combined with transform coding, but
is unlikely to repair q70's 22-point worst SSIMULACRA2 deficit by itself. Train
or tune against disjoint images, then validate only through the frozen corpus.

### 3. Lightweight learned analysis/synthesis transform

**Potential: very large. Runtime fit: currently poor.**

JPEG AI and PICO prove that 30% and larger rate reductions are possible. They
also miss Fastvid's sub-millisecond 1080p gates by orders of magnitude. A future
branch would need a performance-constrained architecture search measured on the
L40S, integer/FP16 deterministic inference, parallel ANS substreams, and direct
native planar support. It should follow—not precede—the block-transform result,
which tests the frequency-allocation premise with far less machinery.

### 4. Neural or conventional reconstruction filter

**Potential: medium. Runtime fit: uncertain.**

A postfilter may repair SSIMULACRA2 at q70/q80 without changing encoded
coefficients. It cannot use source-only information at decode, must not invent
texture that hurts Butteraugli, and must fit inside the 0.5 ms RGB decode budget.
A separable 3x3 or block-local shrinkage filter is more credible than a CNN.

### 5. Richer intra prediction and cross-component tools

**Potential: small to medium alone. Runtime fit: good.**

Fastvid's prior affine chroma-from-luma model saved only 0.109% whole-stream and
its lossless squeeze saved 0.801%. Larger local CfL, palette, directional, and
screen-content tools remain useful fallbacks, but existing evidence does not
support them as the primary route to a 38% byte reduction.

## Revised experimental sequence

1. Supersede the sub-64 KiB neural entropy classifier as the immediate branch;
   its predeclared 0.5% target is immaterial to 10x.
2. Implement a single GPU 8x8 frequency-transform mode with fixed perceptual
   quantization and exact spatial fallback.
3. Compare unchanged q90 canonical rejection artifacts. Reject on any gate or
   absent byte improvement; promote unchanged code to full only after a pass.
