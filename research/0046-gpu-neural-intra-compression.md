# GPU-native neural intra compression

Date: 2026-07-29

## Question

Can a learned image codec improve Fastvid's compression while retaining
independent frames, the required 8/10/16-bit planar formats, perceptual quality,
and the canonical CUDA latency and throughput gates?

## Sources reviewed

- Kang et al., [PILC: Practical Image Lossless Compression with an End-to-end
  GPU Oriented Neural Framework](https://openaccess.thecvf.com/content/CVPR2022/html/Kang_PILC_Practical_Image_Lossless_Compression_With_an_End-to-End_GPU_Oriented_CVPR_2022_paper.html),
  CVPR 2022. The paper combines a lightweight autoregressive model, VQ-VAE, and
  a GPU entropy coder. It reports 200 MB/s encode and decode on a Tesla V100 and
  about 30% fewer bytes than PNG. The paper and supplemental material are open;
  no maintained reference implementation was found in the reviewed sources.
- He et al., [Checkerboard Context Model for Efficient Learned Image
  Compression](https://openaccess.thecvf.com/content/CVPR2021/html/He_Checkerboard_Context_Model_for_Efficient_Learned_Image_Compression_CVPR_2021_paper.html),
  CVPR 2021. A two-pass anchor/non-anchor schedule makes spatial latent context
  parallel and reports over 40x faster decoding than serial context with nearly
  unchanged rate-distortion performance.
- He et al., [ELIC: Efficient Learned Image Compression with Unevenly Grouped
  Space-Channel Contextual Adaptive
  Coding](https://openaccess.thecvf.com/content/CVPR2022/papers/He_ELIC_Efficient_Learned_Image_Compression_With_Unevenly_Grouped_Space-Channel_Contextual_CVPR_2022_paper.pdf),
  CVPR 2022. Uneven channel groups concentrate bits in early latent channels;
  the parallel variant combines this with checkerboard context. It is a useful
  transform/context design, but its multi-stage analysis/hyperprior/synthesis
  pipeline is much larger than Fastvid's current kernels.
- Barzen et al., [Accelerated Deep Lossless Image Coding with Unified
  Parallelized GPU Coding Architecture](https://arxiv.org/abs/2207.05152),
  2022. A small per-pixel density network is scheduled on a wavefront and beats
  JPEG 2000/FLIF in its test, but only reaches sub-second coding for 8-bit gray
  images on a GTX 1070 Ti. The paper reports that matrix multiplication consumes
  about 75% of runtime.
- Zheng et al., [RAWIC: Bit-Depth Adaptive Lossless Raw Image
  Compression](https://arxiv.org/abs/2603.28105), 2026. Conditioning an
  ELIC-derived entropy model on patch bit depth improves high-bit-depth Bayer
  compression and averages 7.7% fewer bits than JPEG XL. Its reported runtime
  at 2048x1536 is 6.71 s encode and 10.72 s decode; the authors explicitly leave
  real-time latency reduction as future work. Bayer raw is also not the required
  YUV422/RGB444/gray matrix.
- InterDigital, [CompressAI](https://github.com/InterDigitalInc/CompressAI),
  BSD-3-Clause-Clear. This is the most mature open PyTorch learned-compression
  framework reviewed, with pretrained image models and entropy operations. It
  is suitable for offline model exploration and training, not a ready Fastvid
  bitstream implementation.

## Gate-derived feasibility

Fastvid's evaluator requires 4Kx24 encode throughput of at least 2.0 GP/s for
YUV422 and 1.5 GP/s for RGB444, with decode minima of 3.0 and 2.0 GP/s. It also
requires single-frame 1080p RGB444 below 1.0 ms encode and 0.5 ms decode.

Even PILC's exceptional 200 MB/s result is roughly an order of magnitude below
the input-byte throughput implied by the 4K gates and hundreds of times above
the allowed 1080p latency once startup and tensor traffic are included. RAWIC,
DLIC, ordinary hyperprior autoencoders, and serial PixelCNN-style models are
not plausible candidates under these gates. GPU residency removes PCIe copies;
it does not remove convolution, activation, latent, probability-table, and
entropy-serialization costs.

## Actionable architecture space

### Reject for the first branch

- Full analysis/hyperprior/synthesis autoencoders. They replace the codec rather
  than improve its measured bottleneck and have no credible path to 0.5 ms RGB
  decode at the required resolution.
- Per-pixel autoregression. Fastvid already pays for one shallow causal
  wavefront; adding a neural inference at every diagonal multiplies its serial
  span and makes entropy probabilities part of the decode dependency.
- Per-image model fitting or transmitted weights. Encoding latency and model
  overhead are incompatible with the API and single-frame latency gate.
- Generative/perception-only objectives. Fastvid gates every frame with both
  SSIMULACRA2 and Butteraugli; hallucinated detail cannot substitute for the
  source.

### Retain

1. **Fixed tiny checkerboard predictor.** Predict non-anchor samples from
   anchors with one or two integer/FP16 3x3 layers, then code anchors and
   residuals as independent parallel shards. Two passes preserve decode
   parallelism. A normative fixed model must be identical for all machines and
   all weights must ship with the decoder.
2. **Neural mode/scale selector, conventional reconstruction.** Run a small
   tile-level network once to select predictor and entropy parameters while the
   actual reconstruction remains integer and explicit. This has lower upside
   but keeps correctness, q100 exactness, and entropy decoding simple.
3. **Learned residual entropy parameters with a mandatory conventional
   fallback.** Predict a small set of distribution classes per shard, charge the
   class id and tables, and retain Rice/block-pack/order-0 when the neural model
   does not save complete bytes. This can never worsen chosen size except for
   selector overhead, and it isolates neural inference from reconstruction.

The third option is the safest first implementation because Fastvid can compare
complete encoded byte counts before selecting it. It also allows model inference
for all shards in a small number of batched CUDA launches and leaves existing
quality unchanged.

## Training and format constraints

- Do not train on canonical evaluation frames. Freeze a disjoint, licensed
  training corpus and record its source checksums and preprocessing.
- Condition explicitly on layout, plane, declared bit depth, quantizer step,
  and simple shard statistics. One model must cover the full required matrix.
- Quantize weights and activations deterministically. Decoder-visible
  probability tables must be integer-exact; floating-point output cannot define
  a bitstream decision differently across devices.
- Include weight storage, initialization, workspace, and kernel launches in the
