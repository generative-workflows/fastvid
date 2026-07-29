# Fastvid

These instructions are edited by the user and MUST NOT be edited unless specifically requested.
Evaluation logic also MUST NOT be edited unless specifically requested.

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

## Canonical Evaluation Harness

`scripts/evaluate.py` is the one canonical entry point for testing codec
changes.

It provides quality and performance gates.

The evaluator may call internal helpers, metric binaries, build tools, and
profilers, but researchers and coding agents must invoke evaluation through
this entry point. Do not create experiment-specific benchmark or quality
scripts, and do not accept results from ad-hoc commands.

The evaluator provides a fast `rejection` tier on a fixed, representative corpus subset and a `full` tier over the entire source corpus with frozen strata for format/depth.

The rejection tier exists only to shorten feedback. Passing it is not an
acceptance result. A candidate may be accepted only after the unchanged
candidate passes the full tier.

## Compression Objective

After all gates pass, maximize corpus-wide compression.

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
9. Commit between experiments.

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
