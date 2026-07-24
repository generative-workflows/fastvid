# Context-conditioned residual entropy after tile-local ANS

## Question and forward-citation path

EXP-0055 removes much of the mismatch between Fastvid residual histograms and
Rice/zero-run codes, but it still assumes that every residual in a tile is
drawn from one memoryless distribution. This pass follows recent work that
cites or builds on ANS, JPEG XL, and spatial probability models to ask which
conditional model could save bytes without sacrificing tile access or
requiring a learned runtime.

## Open sources

- Duda, [*Exploiting context dependence for image compression with
  upsampling*][duda-context], 2020 preprint.
- Rhatushnyak et al., [*Committee Draft of JPEG XL Image Coding
  System*][jxl-paper], 2019 preprint.
- libjxl, [modular encode-effort documentation][jxl-effort] and reference
  implementation, BSD-3-Clause.
- Mentzer et al., [*Practical Full Resolution Learned Lossless Image
  Compression*][l3c], CVPR 2019 open-access paper.
- Pieprzyk et al., [*The Compression Optimality of Asymmetric Numeral
  Systems*][ans-optimality], 2023, CC BY 4.0.

[duda-context]: https://arxiv.org/abs/2004.03391
[jxl-paper]: https://arxiv.org/abs/1908.03565
[jxl-effort]: https://github.com/libjxl/libjxl/blob/main/doc/encode_effort.md
[l3c]: https://openaccess.thecvf.com/content_CVPR_2019/html/Mentzer_Practical_Full_Resolution_Learned_Lossless_Image_Compression_CVPR_2019_paper.html
[ans-optimality]: https://doi.org/10.3390/e25040672

These are design references, not a claim that every technique mentioned in a
standard or paper is unencumbered. The proposed first experiment is only a
probability model and introduces no external code.

## Findings relevant to Fastvid

Duda models not only the conditional center of a residual distribution but
also its width from already decoded local gradients. On 48 grayscale images,
the paper reports 0.645 bits/difference average savings for the final
upsampling pass, including 0.393 bits/difference from center prediction. Its
important low-complexity observation is that neighboring absolute
differences predict local scale: smooth and textured regions should not share
one residual distribution.

JPEG XL's modular effort ladder provides production evidence for the same
direction. Its first ANS level uses a fixed meta-adaptive context based on
gradient error; higher efforts add weighted-predictor error, learned context
trees, previous-channel properties, and more exhaustive context clustering.
It also shows a useful engineering split: fixed contexts are the cheap
feedback candidate, while learned trees are a slower compression frontier.

L3C is not suitable for Fastvid's current CPU/safety/dependency constraints,
but its parallel hierarchy is a counterexample to assuming that every strong
conditional model must be pixel-serial. Conditioning can be computed in
coarse or previously decoded groups. Fastvid should first test a fixed causal
context, not a neural model.

The 2023 ANS optimality analysis explicitly separates coder redundancy from
auxiliary statistics, final state, and stream-length costs, and notes that
auxiliary cost is material for short sequences. Splitting one tile histogram
into many contexts can therefore lose even when conditional entropy improves.
Every candidate must charge all context tables and signaling.

## Cheapest applicable oracle

The first oracle should use only the previous decoded folded residual within
the same tile:

```text
context 0: previous folded residual == 0
context 1: previous folded residual in 1..threshold
context 2: previous folded residual > threshold
```

The first sample uses context zero. This is causal, deterministic, independent
of adjacent tiles, available before decoding the current symbol, and much
cheaper to model than a spatial context tree. Thresholds 1, 3, 7, and 15
cover useful local-scale partitions.

For each threshold, charge:

- one complete normalized table and terminal state per nonempty context;
- context-count and threshold signaling;
- byte rounding independently per rANS state; and
- exact fallback to the version-3 single-table mode.

Because rANS encoding processes symbols in reverse, the encoder may precompute
the causal context labels in a forward pass and then encode each context
substream backward. The decoder consumes the appropriate context stream
according to the preceding decoded residual. A later format would need
substream lengths or a specified interleaving; the oracle must charge that
framing before acceptance.

## Next directions if the cheap oracle wins

1. Replace previous-residual magnitude with a bucket of the causal spatial
   predictor error or local gradient magnitude, closer to Duda and JPEG XL.
2. Cluster per-context histograms when table overhead erases conditional
   entropy gains.
3. Test already-decoded luma magnitude as context for chroma residual width.
4. Separately model reversible squeeze/color transforms; they alter the
   residual source and should not be conflated with entropy conditioning.

Learned context trees, per-image regression coefficients, and large
autoregressive neighborhoods remain deferred until fixed contexts show a
complete-byte opportunity.

## Relevant experiments

- [EXP-0053: finite-block order-0
  model](../experiments/EXP-0053-finite-block-order0-model.md)
- [EXP-0055: modeled rANS
  selector](../experiments/EXP-0055-modeled-rans-selector.md)
- [EXP-0056: causal context-conditioned order-0
  model](../experiments/EXP-0056-causal-context-order0-model.md)
