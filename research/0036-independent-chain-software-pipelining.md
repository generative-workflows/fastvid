# Independent-chain software pipelining

## Question

Can Fastvid expose instruction-level parallelism by interleaving two
independent causal tile chains without changing either chain's result?

## Sources

- AMD, [*Software Optimization Guide for AMD Family 15h
  Processors*](https://www.amd.com/content/dam/amd/en/documents/archived-tech-docs/software-optimization-guides/47414_15h_sw_opt_guide.pdf),
  Chapter 8.
- LLVM,
  [`MachinePipeliner.cpp`](https://llvm.org/doxygen/MachinePipeliner_8cpp_source.html),
  Apache-2.0 WITH LLVM-exception.
- Kiriansky et al.,
  [*Cimple: Instruction and Memory Level
  Parallelism*](https://arxiv.org/abs/1807.01624), 2018.
- Intel,
  [*Intel 64 and IA-32 Architectures Optimization Reference
  Manual*](https://www.intel.com/content/www/us/en/developer/articles/technical/intel64-and-ia32-architectures-optimization.html),
  current manual index.

## Findings

AMD's scheduling guidance identifies long dependency chains as a throughput
limit and recommends loop unrolling where it exposes independent operations.
LLVM's machine pipeliner formalizes the compiler version: schedule operations
from different logical iterations into a pipelined kernel, with prologue and
epilogue handling. Cimple generalizes the idea to interleave independent
state machines and reports that explicit interleaving can expose both
instruction- and memory-level parallelism that one chain cannot.

Fastvid's reconstructed-neighbor clamp-gradient loop cannot be unrolled into
independent iterations within one tile: sample `x + 1` needs reconstructed
sample `x`. Adjacent tiles, however, are format-defined independent units.
Their source rows, reconstruction rows, entropy payloads, and writer states
do not alias. Alternating operations from two tiles therefore preserves the
exact per-tile order while giving the out-of-order CPU two quantizer-load and
reconstruction chains to schedule.

This is scalar software pipelining, not SIMD. It is relevant precisely
because the predictor blocks horizontal vectorization. The approach also
differs from multi-threading: one worker processes two independent states,
so it can improve the one-thread OpenAPV comparison without synchronization.

## Fastvid boundary

The matched all-intra q90 frame at standard 256x128 geometry has 90 tiles:
30 Rice-0 luma tiles, 30 zero-run chroma tiles, and 30 block-packed chroma
tiles. A paired Rice kernel can therefore cover one third of all samples and
the mode responsible for the 28.80% scalar-writer profile share.

Only equal-size, same-plane tiles with the same sampled Rice-0 or Rice-4
selection should enter the first prototype. Every other pair retains the
accepted scalar path. Pair scheduling must return payloads in original
directory order and prove byte equality against two independent scalar
encodes.

## Relevant experiments

- [EXP-0081](../experiments/EXP-0081-above-predictor-screen.md)
- [EXP-0090](../experiments/EXP-0090-post-pack-speed-profile.md)
- [EXP-0097](../experiments/EXP-0097-post-rice4-profile.md)
- [EXP-0099](../experiments/EXP-0099-interleaved-rice-tile-pairs.md)
