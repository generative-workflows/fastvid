# Edge-GPU predictive compression

## Source and use constraints

Oscar Ferraz, Vitor Silva, and Gabriel Falcao,
[*Hyperspectral Parallel Image Compression on Edge
GPUs*](https://doi.org/10.3390/rs13061077), Remote Sensing 13(6), 1077,
2021.

The article is distributed under
[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). This record uses
the paper's published architecture and measurements with attribution. No
source code or normative CCSDS-123 syntax is copied, and Fastvid does not rely
on the license of the authors' C/CUDA implementation.

CCSDS-123 hyperspectral lossless compression is not a matched Fastvid
benchmark. Its many spectral bands expose different independence from
three-plane YUV video. The work is useful here as evidence about dependency
isolation, heterogeneous method selection, and variable-size output costs.

## Architecture

The predictor contains both sample-parallel stages and a circular dependency
in its adaptive weights. The implementation isolates the dependency in one
kernel, which is only band-parallel, while surrounding work uses
sample-parallel kernels. In a reduced predictor mode the weights dependency is
removed and sample-level parallelism becomes available throughout. Shared
memory, vectorized memory transactions, and CUDA streams are used to improve
locality and overlap transfer with execution.

This gives two distinct design options rather than a vague instruction to
"parallelize prediction":

1. isolate the shortest unavoidable recurrence and make all surrounding
   transforms data-parallel; or
2. retain a simpler predictor frontier point whose weaker dependency buys
   substantially more hardware parallelism.

Fastvid version 5 follows the second option at the format level with fixed
clamp-gradient prediction and bounded independent entropy shards. On a scalar
CPU, two independent tiles can also be interleaved to expose instruction-level
parallelism without changing either causal chain.

## Entropy and output assembly

The block-adaptive encoder evaluates zero-block, second-extension, and sample
splitting methods. Fourteen sample-splitting parameters are independent and
are evaluated on the GPU; CPUs handle the less parallel methods and construct
partial bitstreams. The reported final concatenation is serial:

- under 3% of sample-adaptive execution;
- about 5% of block-adaptive time on Nano and TX2; and
- about 10% on Xavier.

Method execution accounts for roughly 45% of block-adaptive time, while
bitstream generation accounts for approximately 35–50%. The paper also
reports gains from fixed parameters, avoiding unnecessary variable-length
writes, and reusing data through L2.

Fastvid should copy the decomposition, not the serial concatenation. Version
5 already computes exact Rice lane sizes, assigns disjoint output spans, and
writes the final Rice body without per-lane collection. A CUDA implementation
should generalize that design as:

1. compute candidate costs and selected sizes independently;
2. exclusive-scan shard sizes;
3. write selected methods directly to canonical disjoint spans.

This keeps output assembly logarithmic-span rather than assigning one CPU
core to concatenate every partial stream.

## Reported performance and interpretation

The paper reports over one gigasample per second for prediction, complete
system throughput above one gigabit per second, up to 611 Mb/s/W, and as much
as 150x speedup over its serial Nano baseline. Its largest block-adaptive
speedup is 71x on Xavier. Those values demonstrate scaling of the
decomposition on Jetson hardware; they are not rate, quality, or speed targets
for Fastvid because the source layout, codec, and hardware differ.

The more transferable measurements are the stage fractions. As the parallel
methods get faster, serial concatenation grows to 10% of total time. A future
Fastvid CUDA benchmark must therefore record predictor, cost selection, scan,
emission, and host/device transfer separately, as well as complete codec
throughput.

## Fastvid consequences

- Preserve version 5's four normative entropy lanes and 4,096-symbol restart
  boundary; do not collapse them into one serial tile stream.
- Interleave independent causal tile chains on scalar CPUs and assign chains
  independently on GPUs.
- Keep predictor residual staging separate from entropy cost and emission so
  each stage can acquire its own hardware mapping.
- Evaluate entropy candidates concurrently on wide hardware, but charge every
  mode byte, lane length, padding byte, and output offset.
- Use exact sizes plus scan/disjoint writes rather than a mutex or a final
  single-core append.
- Measure the longest causal chain and output-assembly span, not only aggregate
  MP/s.

## Relevant experiments

- [EXP-0100](../experiments/EXP-0100-parallel-serialization-budget.md)
- [EXP-0108](../experiments/EXP-0108-bounded-shard-stream-prototype.md)
- [EXP-0110](../experiments/EXP-0110-full-tile-bounded-shards.md)
- [EXP-0123](../experiments/EXP-0123-matched-direct-emission-isolation.md)
- [EXP-0126](../experiments/EXP-0126-selector-fused-zero-run-cost.md)
- [EXP-0129](../experiments/EXP-0129-interleaved-full-tile-predictors.md)
