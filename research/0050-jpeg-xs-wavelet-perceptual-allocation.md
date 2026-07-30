# JPEG XS-style wavelet and perceptual allocation

Date: 2026-07-30

## Question

Which intra-frame architecture has a credible path from Fastvid's corrected
6.188x rejection baseline toward 10x while retaining native CUDA operation,
independent frames, high bit depths, and strict latency gates?

## Primary-source findings

- The JPEG Committee's [JPEG XS overview](https://jpeg.org/jpegxs/) describes
  a low-complexity, few-line-latency codec optimized for visually lossless
  quality, precise rate control, native 4:4:4 and 4:2:2 sampling, and typical
  compression ratios up to 10:1.
- Descampe et al., [JPEG XS, a new standard for visually lossless low-latency
  lightweight image coding](https://dial.uclouvain.be/pr/boreal/object/boreal%3A248275),
  describe its shallow wavelet, precinct, magnitude-level, and refinement
  architecture and real-time CPU/GPU implementations.
- Brummer and de Vleeschouwer, [Adapting JPEG XS Gains and Priorities to Tasks
  and Contents](https://openaccess.thecvf.com/content_CVPRW_2020/html/w7/Brummer_Adapting_JPEG_XS_Gains_and_Priorities_to_Tasks_and_Contents_CVPRW_2020_paper.html),
  show that fixed band gains and priorities can be optimized offline.
- Alakuijala et al., [Guetzli: Perceptually Guided JPEG
  Encoder](https://arxiv.org/abs/1703.04421), report 29--45% size reductions at
  matched Butteraugli distance by optimizing quantization and DCT coefficient
  choices. Guetzli is too slow, but a small parallel candidate set can retain
  the encoder-only perceptual-allocation principle.
- The JPEG Committee reports that [JPEG AI became an International
  Standard](https://jpeg.org/items/20250219_press.html) with about 30% coding
  improvement over advanced conventional anchors. Its full learned path is not
  credible under Fastvid's sub-millisecond decode gate and lacks direct required
  16-bit/4:2:2 coverage.

## Actionable design

The strongest large-gain branch is a shallow integer lifting wavelet with small
independent precincts, fixed per-band gains, and parallel significance,
magnitude, and refinement streams. Start with one fixed transform and gain
table plus the current spatial path as a complete-byte fallback. Later evaluate
two to four signaled quantizer classes per precinct in parallel and choose the
smallest class satisfying a calibrated local error bound.

This differs from EXP-0172, which tested a fixed Walsh-Hadamard transform with
uniform coefficient treatment, expanded overlapping bytes by 22.26%, and did
not test frequency-selective wavelet quantization or precinct allocation.

Full hyperprior, transformer, generative, per-image-fitting, and serial
autoregressive codecs are outside the measured latency envelope. A neural
contribution should be limited to an offline teacher or tiny deterministic
precinct selector; normative reconstruction and tables must remain exact.

## Experimental order

1. Repair the failing baseline using MED plus absolute-lattice reconstruction
   (EXP-0183).
2. Test one shallow integer wavelet precinct mode with fixed frequency gains
   and exact current-path fallback.
3. Add bounded perceptual precinct-class selection only after a rate benefit.

Ten-fold compression remains unproven under Fastvid's corrected Butteraugli,
SSIMULACRA2, edit-generation, and performance gates.
