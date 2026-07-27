Fastvid is a GPU-accelerated intermediate codec designed for high-resolution video.

It balances three goals simultaneously:
    1. Perceptual Similarity
    2. Encoding and Decoding Speed
    3. Compression Ratio

This project is based around experimental research and formal verification.
Every experiment MUST be logged (see experiments section below).
Every research artifact MUST be logged (see research section below).
Evaluation methodology MUST be clearly defined by research and experiments, so fastvid has a clear target to optimize against (see evaluation methodology below).

## Implementation

This is a fast intermediate video codec consisting of two reference implementations: 
  1. A CPU reference, written in Rust
  2. A GPU reference, written as a C++/CUDA custom extension for pytorch.

The python API should be extremely simple. For encoding, accept a 16-bit RGB and YUV tensor and encode as 4:2:2 or 4:4:4. For images, support a shape of [3, H, W] or [3, 1, H, W]. For videos, accept [3, T, H, W].

Decoding should allow decoding from either VRAM or DRAM. Decoding yields
tensors on CUDA. The decode API should support decoding everything, as well as decoding selected frames or a range of frames from a video.

Fastvid supports tiled encoding and decoding for use-cases where tile-wise edits are possible with
lower resident VRAM.

### Engineering Requirements

The codec should be largely size and length-independent, supporting arbitrary (rational) framerates.
It should be possible to relatively cheaply decode or re-encode individual frames and tiles, with some overhead acceptable (e.g. needing to decode a few adjacent frames). This is necessary for downstream usage in video editing tasks.

Some concrete goals to reach simultaneously:
  1. >5 GP/s of decoding for ~real-time 4K.
  2. >3 GP/s of encoding for ~real-time 4K.
  3. >50dB minimum per-frame XPSNR
  4. >15x compression ratio.

### Patent Free / Open Source

Do not draw on sources that are not open for use. Require MIT, Apache, or compatible licenses.
This project will be legitimately MIT licensed and needs to have the grounding in its dependencies
to do so.

### Draw on Research Literature

Most of the codecs this will be measured against were written over 15 years ago. There is surely an advanced literature describing new compression techniques, optimized implementation, and metrics. Use code and research that is openly available to create the best possible implementation, with attribution.

Keep track of research references in the `research` subdirectory, using `research/INDEX.md` as an index. Independently verify results and try many paths. Each research reference should have a dedicated file, e.g. `research/0001-some-paper.md` that catalogues key findings and insights.

Research references should contain a section linking them to relevant experiments.

Code that is based on research should link to the research reference.

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

Experiments, after completion, are IMMUTABLE RECORDS.

## Perceptual Similarity

This is an engineering goal rather than a hard metric. However, concrete metrics like XPSNR, SSIM, 
VMAF, and per-pixel error should be used to measure error rates concretely.

We don't need to use "lossless" compression, but we do need to avoid as much perceptual destruction
as possible. This is an intermediate codec like ProRes, which is designed for high-fidelity editing
and storage.

## Evaluation Methodology

The evaluation methodology and metrics should be clearly defined and backed by research.
Keep a record in `EVALUATION_METHODOLOGY.md` with links to research.

This methodology defines clear targets to optimize against: quality, speed, and compression ratios.
Experiments MAY diverge from the standard methodology.

Evaluation methodology may evolve over time as new research is discovered and performed.

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

## System Limits

Take note of the CPU and memory limits of the current machine and structure benchmarks accordingly.
