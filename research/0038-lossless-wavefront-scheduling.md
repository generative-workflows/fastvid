# Lossless wavefront scheduling and entropy handoff

## Question

How can Fastvid parallelize a causal spatial predictor without changing its
rate-distortion behavior, and where should the predictor hand residuals to a
parallel entropy coder?

## Sources

- Murai, Lin, Arunruangsirilert, and Katto,
  [*Wavefront Parallelization for Efficient Learned Image
  Compression*](https://arxiv.org/abs/2607.19082), MMSP 2026.
- Barzen, Glazov, Geistert, and Sikora,
  [*Accelerated Deep Lossless Image Coding with Unified Parallelized GPU
  Coding Architecture*](https://arxiv.org/abs/2207.05152), 2022.
- Kang et al.,
  [*PILC: Practical Image Lossless Compression with an End-to-end GPU
  Oriented Neural Framework*](https://openaccess.thecvf.com/content/CVPR2022/html/Kang_PILC_Practical_Image_Lossless_Compression_With_an_End-to-End_GPU_Oriented_CVPR_2022_paper.html),
  CVPR 2022.
- Meta,
  [DietGPU](https://github.com/facebookresearch/dietgpu), BSD-3-Clause
  source for massively parallel generalized ANS.

The 2026 wavefront paper is openly readable but its promised implementation
was not available when reviewed on 2026-07-26. It is used as scheduling
evidence, not copied code. DLIC and PILC are comparative research evidence;
no unlicensed implementation material is used. DietGPU is compatible
implementation evidence.

## Findings

### Preserve the dependency graph instead of approximating it

Murai et al. apply a hyperplane schedule to an existing causal context and
report more than 13x acceleration without retraining or rate-distortion
change. Their central distinction is useful beyond learned codecs:
reordering evaluations that are independent in the original dependency DAG
is lossless, while checkerboard/context approximation changes the model.

For Fastvid clamp-gradient, sample `(x, y)` depends on `(x-1, y)`,
`(x, y-1)`, and `(x-1, y-1)`. Therefore all samples with equal `x + y` are
independent, and the exact full-tile schedule has `width + height - 1`
rounds. EXP-0105 measured 383 rounds for a 256x128 tile, replacing a
32,768-step scalar predictor span without predictor restarts or a rate
change.

DLIC independently demonstrates the same system shape for lossless coding:
causal neighborhoods advance as a GPU wavefront, and the paper extends the
schedule across image slices. This supports treating frame and tile axes as
additional scheduling dimensions when their dependencies permit it.

### The entropy handoff is separate from predictor scheduling

A dependency-preserving schedule does not require a corresponding bitstream
order. EXP-0106 confirms this distinction empirically. Anti-diagonal
serialization leaves Rice bit count unchanged but damages spatial grouping
for zero-run and fixed-block modes: +2.66% aggregate and up to +29.39% on the
representative q90 screen.

The predictor should instead write each residual to its canonical raster
index in a tile-local staging array. Entropy shards then consume contiguous
raster ranges. The staging writes are scattered along one wavefront, but
each address is unique and no atomic operation is needed. A later kernel can
read contiguous shards coalescently. This costs one bounded residual buffer,
not a format change.

PILC reinforces that a fast GPU codec must co-design the probability model
and entropy backend: its reported 200 MB/s depends on a deliberately
low-complexity coder rather than leaving a serial coder after parallel
prediction. DietGPU demonstrates the complementary implementation pattern:
many explicit ANS states and batched device-resident buffers reach GPU-scale
throughput.

## Fastvid design consequences

1. Keep full-tile wavefront prediction as the zero-rate branch.
2. Preserve raster residual syntax; do not couple execution order to storage
   order.
3. Materialize at most one tile/band of folded residuals before entropy
   coding. For a 256x128 luma tile this is 64 KiB with `u16` folded residuals
   or 128 KiB with `u32`; a 64-row band halves those bounds.
4. Consume the staging array through fixed 4,096-symbol raster shards.
5. Give Rice shards four independent lanes; keep block-pack at its existing
   128-symbol boundary and explicitly shard zero-run streams as well.
6. Model the combined predictor-boundary, entropy-lane, shard-length, and
   padding costs before defining syntax.
7. On CUDA, separately time prediction, staging, entropy, prefix scan, final
   write, and transfers. A wavefront-only kernel timing is not an end-to-end
   codec result.

## Relevant experiments

- [EXP-0102](../experiments/EXP-0102-four-lane-rice-shard-model.md)
- [EXP-0104](../experiments/EXP-0104-predictor-band-height-ladder.md)
- [EXP-0105](../experiments/EXP-0105-predictor-wavefront-model.md)
- [EXP-0106](../experiments/EXP-0106-diagonal-residual-order-model.md)
- [EXP-0107](../experiments/EXP-0107-combined-wavefront-entropy-model.md)
