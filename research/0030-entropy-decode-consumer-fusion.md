# Entropy decode consumer fusion and the SIMD boundary

## Question

Fastvid's order-0 decoder first materializes every folded residual and then
walks that temporary allocation to reconstruct a tile. This note asks whether
open reference implementations support direct consumption, and where SIMD
would require a format change rather than a local implementation trick.

## Open implementation sources

- Fabian Giesen, [`ryg_rans`](https://github.com/rygorous/ryg_rans), public
  domain.
- Fabian Giesen, [*Interleaved Entropy
  Coders*](https://arxiv.org/abs/1402.3392), 2014 preprint.
- Yann Collet,
  [`FiniteStateEntropy`](https://github.com/Cyan4973/FiniteStateEntropy),
  BSD-2-Clause.

Giesen's scalar example obtains one symbol from the current rANS state, writes
it immediately to the destination, and advances the state. It does not require
an intermediate symbol stream. Fastvid's reconstruction has a causal spatial
dependency, so consuming each decoded residual immediately also shortens the
live working set: only the output tile and small decoding table remain.

This is an implementation property, not a new entropy format. A generic
consumer callback can preserve the existing symbol-vector helper for models
and tests while allowing production decode to reconstruct directly.

## SIMD boundary

A single rANS state is a recurrence: symbol lookup determines the frequency
needed to compute the next state. Giesen's interleaving work obtains
instruction-level and SIMD parallelism by coding multiple independent states.
The accompanying public-domain implementation describes an SSE 4.1 decoder
requiring at least four independent streams, and its example emits a different
interleaved stream.

Therefore:

- fusing scalar decode and reconstruction can be byte-identical;
- explicit SIMD of the entropy state transition is not a drop-in loop rewrite;
- a SIMD experiment must specify state count, tail handling, stream syntax,
  rate overhead, and scalar fallback as a new mode; and
- SIMD is attractive only after a byte-charged model shows that multiple final
  states and alignment do not erase the speed benefit on Fastvid's finite
  tiles.

FiniteStateEntropy likewise exposes symbol-at-a-time state operations and
multiple decoding states, reinforcing the distinction between direct output
and multi-state parallelism. Its implementation is a design reference only;
Fastvid does not copy code from it.

## Relevant experiments

- [EXP-0053: finite-block order-0 model](../experiments/EXP-0053-finite-block-order0-model.md)
- [EXP-0055: modeled rANS selector](../experiments/EXP-0055-modeled-rans-selector.md)
- [EXP-0066: maximum-compression kernel profile](../experiments/EXP-0066-maximum-compression-profile.md)
- [EXP-0067: fused rANS reconstruction](../experiments/EXP-0067-fused-rans-reconstruction.md)
