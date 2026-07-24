# Streaming Rice parameter selection

## Question

EXP-0062 finds that residual construction plus entropy finalization consumes
about 42% of the fixed-gradient speed-tier profile. Fastvid currently stores
every folded residual, builds exact zero-run and Rice statistics, chooses a
mode, and traverses the residuals again. Can an open, causal parameter
estimator preserve the existing Rice syntax while avoiding that buffer and
second pass?

## Open sources

- Solano Donado, [*On the Optimal Calculation of the Rice Coding
  Parameter*](https://doi.org/10.3390/a13080181), Algorithms 2020, CC BY 4.0.
- Team CharLS, [`regular_mode_context.hpp`](https://github.com/team-charls/charls/blob/c0bae6496fa5d787fbb4698debd1e5decb40cf3a/src/regular_mode_context.hpp)
  and
  [`scan_encoder_core.hpp`](https://github.com/team-charls/charls/blob/c0bae6496fa5d787fbb4698debd1e5decb40cf3a/src/scan_encoder_core.hpp)
  at tree `c0bae6496fa5d787fbb4698debd1e5decb40cf3a`, BSD-3-Clause.
- Xiph.Org,
  [FLAC reference implementation](https://github.com/xiph/flac), Xiph
  BSD-style license.

The first Fastvid experiment uses only general Rice-code identities and its
own estimator. It does not copy an external implementation or adopt JPEG-LS
context modeling.

## Findings

For non-negative folded values `x`, a Rice code with parameter `k` costs
`(x >> k) + 1 + k` bits. Solano Donado evaluates parameter calculation for
arbitrary source distributions, shows that the optimal integer parameter is
near a function of the sample mean, and develops single-pass partition
heuristics. The paper also emphasizes that one parameter can be inefficient
when local magnitudes change and that any partition must charge parameter and
delimiter overhead.

CharLS supplies production evidence for truly causal adaptation. Its regular
context keeps an accumulated absolute error `A` and count `N`, computes
approximately `ceil(log2(A/N))`, codes the current mapped error, and only then
updates the statistics. Its 2026 encoder path replaces a small parameter loop
with a leading-zero-count estimate plus one correcting comparison. Decoder
and encoder therefore derive the same parameter without a residual buffer or
transmitted table.

That fully adaptive design would require a new Fastvid entropy mode. A cheaper
first experiment can retain the current bitstream:

1. sample a deterministic sparse subset of source pixels before causal
   reconstruction;
2. estimate complete Rice costs for every current parameter and a zero-run
   proxy;
3. signal the chosen existing entropy mode;
4. perform the real causal reconstruction once and write that mode directly.

Temporal samples have an exact reference predictor. For spatial samples, the
proxy uses source neighbors rather than lossy reconstructed neighbors, so it
is deliberately approximate. Sampling every 16th location caps the prepass at
6.25% of the full predictor work while removing the `u16` residual buffer,
511-bin histogram updates, exact multi-mode cost pass, and second residual
traversal.

## Risks and gates

- Smooth graphics need zero-run selection; a Rice-only fast path could destroy
  the speed tier's rate position.
- Source-neighbor proxy residuals diverge from reconstructed-neighbor
  residuals below q100.
- A wrong Rice parameter is still decodable but may cause large unary codes.
- Sampling must include tile boundaries and use deterministic positions.

The model must therefore measure per-content rate deltas before implementation
and fall back to exact accumulation if the estimated code exceeds a declared
confidence bound. Only a format-preserving estimator is in scope first;
causal adaptive Rice is a separate future format experiment.

## Relevant experiments

- [EXP-0062: speed-tier entropy profile](../experiments/EXP-0062-speed-tier-entropy-profile.md)
- [EXP-0063: sampled streaming Rice](../experiments/EXP-0063-sampled-streaming-rice.md)
