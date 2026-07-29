# Fastvid

Fastvid is an audio/video codec designed and built through autoresearch. Its
video path is a fast, CUDA-accelerated, perceptually lossless intra-frame codec.
Frames must be independently encodable and decodable: do not use temporal
prediction, motion compensation, GOP dependencies, or any other inter-frame
coding tool.

The research objective is:

> Maximize compression while satisfying every correctness, perceptual-quality,
> and performance gate in the canonical evaluator.

The gates are constraints, not values to trade away for a better average.

## Current Scope

Prioritize the CUDA video implementation and its Python-facing API. Do not
spend research time on a CPU implementation or formal verification.

The required video format matrix is:

| Color format | Chroma sampling | Required depths |
| --- | --- | --- |
| YUV | 4:2:2 | 8, 10, and 16 bit |
| RGB | 4:4:4 | 10 and 16 bit |
| Gray | 4:0:0 | 8, 10, and 16 bit |

Do not silently evaluate one format or depth as a proxy for another. Every
required matrix entry must have an explicit correctness and quality result.

Audio is part of Fastvid's overall product direction, but the current
autoresearch loop, corpus, and acceptance gates below apply to the video path.
Do not invent audio acceptance criteria; work on audio only when separately
specified.

The implementation should:

- run encode and decode on CUDA and expose a simple Python API;
- support individual images and batches of independent frames;
- accept and return formats without hidden precision loss or unintended chroma
  conversion;
- keep the bitstream independent of image dimensions where practical and
  support arbitrary image sizes;
- retain tile-local encode/decode as a design goal for editing workloads;
- decode from bitstreams resident in either VRAM or host memory, without
  including file I/O in codec timing.

## Canonical Evaluation Harness

`scripts/evaluate.py` is the one canonical entry point for testing codec
changes. Build or repair it before conducting codec experiments if it does not
yet implement these requirements.

The evaluator may call internal helpers, metric binaries, build tools, and
profilers, but researchers and coding agents must invoke evaluation through
this entry point. Do not create experiment-specific benchmark or quality
scripts, and do not accept results from ad-hoc commands.

The evaluator must provide, at minimum:

- a fast `rejection` tier over a fixed, representative corpus subset;
- a `full` tier over the entire corpus and complete format/depth matrix;
- machine-readable output containing configuration, corpus revision, hardware,
  software revisions, per-frame quality, encoded sizes, and timing samples;
- a non-zero exit status when any required correctness, quality, coverage, or
  performance gate fails;
- reproducible selection: the rejection set, conversions, seeds, and corpus
  manifests must be checked into the repository and versioned;
- validation that every expected sample and required format/depth combination
  was actually evaluated.

The rejection tier exists only to shorten feedback. Passing it is not an
acceptance result. A candidate may be accepted only after the unchanged
candidate passes the full tier.

The evaluator and its thresholds are part of the experimental specification.
Do not weaken, bypass, special-case, or silently modify them to make a codec
change pass. A proposed methodology change must be documented and reviewed as
its own experiment before it is used to judge codec changes.

## Non-Negotiable Quality Gates

Use the direct C API from [Vship](https://codeberg.org/Line-fr/Vship) for its
GPU-accelerated SSIMULACRA2 and Butteraugli implementations. Pin the evaluated
libvship revision, path, and build configuration. Pass original and fastvid
roundtrip planes directly from memory; FFVShip, FFmpeg, FFMS2, media containers,
and persisted metric intermediates are not part of canonical evaluation.

For every decoded frame in every required format/depth case:

- SSIMULACRA2 must be strictly greater than 90.
- Butteraugli must be less than or equal to 1.0.

These are per-frame requirements. Corpus means, percentiles, or aggregate
scores cannot hide a failing frame. Report the minimum SSIMULACRA2 score and
maximum Butteraugli score in summaries, while retaining all per-frame results.

Metric inputs are native-depth pinned-memory copies of the already-loaded
canonical planes and the corresponding fastvid decoded planes. Describe YUV422
as full-range BT.709 with left chroma location; describe RGB as full-range RGB
with BT.709 transfer and primaries. Represent gray without altering samples by
supplying its one plane as each of R, G, and B. Narrow the corpus's uint16
container to uint8 only for declared 8-bit inputs; preserve uint16 samples for
10-bit and 16-bit inputs.

Initialize persistent direct libvship handlers per resolution, format, and depth.
Run SSIMULACRA2 and Butteraugli concurrently and allow independent handlers to
process multiple frames in flight. Retain frame order and map every score back to
its original sample and frame. The evaluator must check dimensions, format
metadata, frame count, decode success, and deterministic round trips before
running perceptual metrics. Codec correctness and CUDA timing remain per sample;
metric parallelism must never turn the per-frame quality gates into aggregate
gates.

“Perceptually lossless” in this project means passing both gates above; it does
not mean mathematically lossless.

## Non-Negotiable Performance Gates

Measure CUDA execution with explicit device synchronization and GPU-appropriate
timing. Include all codec work required for a usable encoded bitstream or
decoded frame. Exclude one-time build/import initialization, corpus file I/O,
and metric computation. Record warm-up policy, repetitions, GPU model, clocks
or power mode when available, CUDA version, and relevant build flags.

For a batch of 24 3840×2160 frames:

| Format | Encode throughput | Decode throughput |
| --- | ---: | ---: |
| 4:4:4 | at least 1.5 GP/s | at least 2.0 GP/s |
| 4:2:2 | at least 2.0 GP/s | at least 3.0 GP/s |

Throughput is full-resolution luma pixels processed per second:
`width × height × frame_count / elapsed_seconds`. Do not multiply the pixel
count by the number of planes or channels.

For one 1920×1080 4:4:4 frame:

- encode latency must be less than 1.0 ms;
- decode latency must be less than 0.5 ms.

Report distributions and the statistic used for gating. Until the evaluator
specifies a stricter policy, gate on the median of at least 20 timed repetitions
after warm-up. Always retain individual timing samples. Benchmark on the
designated reference GPU; results from another GPU are informative but cannot
establish acceptance.

## Compression Objective

After all gates pass, maximize corpus-wide compression. Report at least:

- total uncompressed bytes divided by total encoded bytes;
- bits per full-resolution pixel;
- per-sample encoded size and the worst regressions;
- codec metadata and container overhead.

Never improve the headline ratio by omitting hard samples, changing the corpus,
dropping required formats/depths, or averaging away a quality failure. When two
candidates pass every gate, prefer the smaller total encoded corpus size. Use
speed and implementation complexity as secondary tie-breakers.

## Corpus

Maintain a versioned, checksummed full corpus containing:

- 100–200 4K images;
- at least a few examples of each of: people, video games, rendered imagery,
  noise/static or otherwise difficult material, nature, and animals;
- 20 AI-generated 1080p images.

Categories may overlap, but the manifest must label them and demonstrate
coverage. Include varied texture, lighting, gradients, edges, skin tones,
synthetic graphics, fine text, and high-entropy content. Preserve provenance,
license, generation prompt/settings where applicable, original format, and all
conversion steps.

Corpus assets and dependencies must be compatible with legitimate open-source
use. Prefer permissive or clearly redistributable sources, and do not add data
whose licensing or provenance is uncertain.

Treat original high-bit masters as the corpus source of truth. Prefer camera
RAW, OpenEXR, HDR/RGBE, uncompressed high-bit TIFF, or raw YUV. An 8-bit
tone-mapped image is not a valid reference source for 10-bit or 16-bit
evaluation. PNG may be used sparingly where a content class—such as a captured
open-source game—is not reasonably available in a high-bit master; JPEG is a
discovery proxy, not a preferred canonical source. Follow
`corpus/SOURCE_POLICY.md`.

Enforce diversity by provenance as well as semantic labels: use no more than
two frames from one movie or video sequence, no more than one screenshot from
one game title, no more than one camera RAW from one camera model, and no more
than one HDRI from one source asset. AI images are generated once and their
original bytes and prompts are frozen; evaluation and corpus rebuilds must
never regenerate them.

Select the smaller rejection set from the full corpus and freeze its manifest.
It must represent all content categories and include known worst cases. Do not
change it in response to a particular candidate without treating that change as
an evaluation-methodology experiment.

## Autoresearch Loop

For each codec idea:

1. Research the idea and state a falsifiable hypothesis.
2. Record the baseline by running the canonical rejection tier.
3. Make one attributable change.
4. Run the canonical rejection tier with exactly the same settings.
5. Reject immediately on any correctness, coverage, quality, or speed failure.
6. If it passes and improves compression, run the canonical full tier.
7. Accept only if the full tier passes every gate and improves total encoded
   corpus size against the recorded baseline.
8. Record the result, including failures and artifact paths, before starting
   the next experiment.

No result is valid unless it was produced by `scripts/evaluate.py`. Profilers
and microbenchmarks may diagnose a result, but they cannot replace the
canonical evaluator or establish acceptance.

## Research and Experiment Records

Keep research notes in `research/`, indexed by `research/INDEX.md`. Cite open
papers and implementations, record actionable findings, and link each note to
the experiments that use it. Code derived from research must retain appropriate
attribution and license compatibility.

Keep numbered experimental design records in `experiments/` using names such as
`EXP-0001-short-description.md`. Each record must contain:

- status: `PENDING`, `ACCEPTED`, `REJECTED`, or `SUPERSEDED`;
- hypothesis and rationale;
- exact code revision and canonical evaluator command;
- baseline and candidate machine-readable artifact paths;
- corpus and evaluator revisions;
- quality extrema, timing gate results, and compression delta;
- conclusion and links to related research or experiments.

Completed experiment records are immutable. Corrections or follow-up work
belong in a new linked record.

## Direction

Favor codec designs that map naturally to massively parallel CUDA execution:
independent frames, bounded local dependencies, coalesced memory access,
predictable control flow, and parallel output assembly. A compression idea that
cannot meet the fixed throughput and latency gates is not a Fastvid direction,
regardless of its offline compression ratio.

The JPEG XL Pareto-front discussion is useful framing:
<https://cloudinary.com/blog/jpeg-xl-and-the-pareto-front>. Fastvid seeks the
best compression point inside its fixed quality and speed constraints, not a
single metric win outside them.
