# Multi-generation quantization drift

Date: 2026-07-30

## Sources and finding

Li, Yang, and Wang, [Improving Multi-generation Robustness of Learned Image
Compression](https://arxiv.org/abs/2210.17039), identify context-dependent
"corrected quantization" as a source of drift: a small input perturbation
changes the predicted quantization center and can trigger a causal chain. Their
straight quantization places every latent on a fixed lattice, independent of
the entropy-model mean, and substantially improves repeated recompression.

Richter et al., [Multi-generation-robust Coding with JPEG
XS](https://doi.org/10.1109/ISM.2017.12), treat multi-generation robustness as
a codec design constraint and analyze error sources under recompression.
Zhu and Lin, [Idempotent H.264 intraframe multi-generation
coding](https://doi.org/10.1109/ICASSP.2009.4959763), likewise tie
idempotence to stable prediction modes, quantization, transform inversion,
and clipping behavior.

## Fastvid mapping

Fastvid's original encoder rounds the residual around a predictor computed
from already reconstructed neighbors. The effective sample lattice therefore
depends on causal reconstruction state. EXP-0178 replaces this with absolute
sample quantization followed by lossless prediction of the integer lattice
indices. This is the conventional analogue of straight quantization: entropy
decorrelation no longer selects the reconstruction lattice.

## Canonical evidence

At unchanged q90 steps, EXP-0178:

- reduced rejection generation failures from nine to five;
- passed all ordinary perceptual, correctness, coverage, and timing gates;
- saved 21,500 bytes, improving ratio from 6.188001x to 6.188383x;
- retained failures in YUV8, YUV10, YUV16, RGB10, and gray10.

This falsifies the claim that context-independent quantization alone clears
the edit suite, but supports it as a better foundation. The next experiment
should retain the absolute lattice and refine only the remaining failing
format/depth cells. Unlike mean-gradient allocation, this mechanism improved
both generation behavior and rate without an activity heuristic.

Related experiment: [EXP-0178](../experiments/EXP-0178-absolute-sample-lattice.md).
