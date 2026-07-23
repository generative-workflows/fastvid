Fastvid is an intermediate codec designed for high-resolution video.

It balances three goals simultaneously:
    1. Perceptual Similarity
    2. Encoding and Decoding Speed
    3. Compression Ratio

This project is based around experimental research and formal verification.
Every experiment MUST be logged (see experiments section below).
Every research artifact MUST be logged (see research section below).

## Implementation

This is a Rust project. Rust is chosen for its exceptional speed, zero-cost abstractions, and memory
safety.

The project should expose a library and a binary, with configurable multi-threading. It's acceptable to create smaller sub-crates for core routines.

Avoid unsafe code and C/C++ dependencies.
Safe code is often faster than unsafe code, because it allows LLVM to optimize around invariants.

Where unsafe code must be used, it should be kept to the minimum possible module, because unsafe code pollutes the entire module. Unsafe code should be accompanied by a comment proving that no invariants are violated.

It's OK to draw on Rust libraries, but generally for minimal things, like byteorder, system APIs,
and so on. As a low level project, the standard library should be generally suitable.

### Engineering Requirements

The codec should be largely size and length-independent, supporting arbitrary (rational) framerates.
It should be possible to relatively cheaply decode or re-encode individual frames and tiles, with some overhead acceptable (e.g. needing to decode a few adjacent frames). This is necessary for downstream usage in video editing tasks.

High parallel CPU throughput is desired.

### Optimization Guidelines

Focus on memory access patterns, CPU caches, and memory allocation. Use a data-driven layout oriented around CPU cache lines, fast pre-fetching, minimizing copies.

Trade off memory for speed where reasonable, but aim for low memory amplification.

### Patent Free / Open Source

Do not draw on sources that are not open for use. Require MIT, Apache, or compatible licenses.
This project will be legitimately MIT licensed and needs to have the grounding in its dependencies
to do so.

### Draw on Research Literature

Most of the codecs this will be measured against were written over 15 years ago. There is surely an advanced literature describing new compression techniques, optimized implementation, and metrics. Use code and research that is openly available to create the best possible implementation, with attribution.

Keep track of research references in the `research` subdirectory, using `research/INDEX.md` as an index. Independently verify results and try many paths. Each research reference should have a dedicated file, e.g. `research/0001-some-paper.md` that catalogues key findings and insights.

## Specification

We want to have a formal specification, described in the `specs` subdirectory, written in Lean.

We will use Lean and Aeneas to formally verify and compare against Rust components that implement parts of the spec. There is no need to have the entire Rust codebase specified, but sub-components should be verified to the fullest extent possible.

## Experimental Approach

Keep Experimental Design Records in the `experiments` directory. Each experiment should have a numbered name (e.g. `EXP-0001-short-desc-here.md`). Experiments should each catalogue:
  - A hypothesis
  - A modification
  - and a test.

Experiments should reference other documentation artifacts: research references and other experiments.

Experiments can be in 4 states:
  - PENDING: untested
  - ACCEPTED: the experimental results have proven successful
  - REJECTED: the experimental results did not give the desired result
  - SUPERSEDED: the experiment was superseded by a later experiment, with a link to that experiment.

## Perceptual Similarity

This is an engineering goal rather than a hard metric. However, concrete metrics like PSNR, SSIM, 
VMAF, and per-pixel error should be used to measure error rates concretely.

We don't need to use "lossless" compression, but we do need to avoid as much perceptual destruction
as possible. This is an intermediate codec like ProRes, which is designed for high-fidelity editing
and storage.

## Encoding and Decoding Speed

We are looking for maximum encoding and decoding speed with high parallel throughput. Focus exclusively on the CPU for now. Benchmarking encoding and decoding on test files should be done
regularly as part of the development loop.

## Benchmarking

We suggest using `criterion` for microbenchmarks and an iperf-based profiler for profiling larger runs.

Keep an updated benchmark table in the README.md

## Compression Ratio

As an intermediate codec, we are aiming to maximize compression only as it trades off against other
goals. 10x compression is a good initial goal, but we will attempt to go further.

Allow the user a light tuning parameter between quality and compression when encoding.

## Color Spaces

Color is the main goal, not alpha. As a hint, consider 422 YUV for a starting point. Having an alpha mode in the future would be interesting.

Also worth having a greyscale-only mode which we can use for encoding masks and alpha channels separately.

## System Limits

Take note of the CPU and memory limits of the current machine and structure benchmarks accordingly.
