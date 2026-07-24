# Fastvid evaluation methodology

Version: 7 (2026-07-24)

This document defines the standard target used to evaluate Fastvid changes.
Experiments may add diagnostics or deliberately diverge, but an optimization
is not a general improvement unless it is measured against this methodology.
The design is grounded in [research 0004](research/0004-codec-evaluation.md),
[research 0006](research/0006-standard-evaluation-methodology.md), and the
modern metric review in
[research 0018](research/0018-modern-perceptual-metrics.md).

## Goals and tracks

Fastvid balances perceptual similarity, CPU throughput, and compression. No
single number is sufficient. Results are reported at every tested quality and
as rate-distortion curves.

There are two coding tracks:

1. **All-intra:** every frame is independently coded. This is the current
   implemented track and the editing/random-access baseline.
2. **Short-GOP video:** temporal coding with a maximum keyframe interval of
   12 frames. This is a target track; results must state seek preroll and the
   maximum number of dependency frames. It must never be mixed with all-intra
   results.

## Standard core corpus

The versioned, machine-readable definition is
[`corpus/manifest.json`](corpus/manifest.json). Corpus v2 is a strict superset
of the archived v1 manifest and checksums. Its codec track contains:

- twelve still frames spanning camera photography, AI-generated detail,
  chroma-only edges, synthetic grids, fine organic detail, text, dark sparse
  content, high-frequency lines, and dense geometry;
- six 24-frame clips spanning natural/rendered motion, deliberately noisy
  camera footage, synthetic UI scrolling, temporally independent grain, and
  hard scene cuts;
- 640x360, 1280x720, 1920x1080, and 3840x2160 dimensions;
- lossless Blender PNGs and CC0 TIFF camera sources, one explicitly
  public-domain camera clip, and a project-owned AI PNG with its exact prompt;
- deterministic procedural source code for graphics, UI-like animation,
  chroma edges, noise/grain, cuts, resolution scaling, HDR, and alpha;
- canonical BT.709 limited-range planar YUV422p8 derivatives with committed
  SHA-256 values.

The capability track additionally contains source-native 3840x2160
BT.2020/PQ YUV444p10 and 1024x1024 RGBA assets. They are not converted into
headline codec scores while the format lacks HDR metadata, greater bit depth,
4:4:4, and alpha. Evaluation records those capabilities as unsupported;
future implementations must benchmark the native assets without silently
tone-mapping or discarding alpha.

The native high-bit supplement is defined by
[`corpus/high-bit-manifest.json`](corpus/high-bit-manifest.json) and its
checksums. It contains deterministic 10- and 12-bit 1080p stills plus a
24-frame 16-bit 720p motion sequence. This is the required correctness and
performance smoke corpus for every high-bit change. It supplements rather
than replaces the broader 8-bit corpus: procedural content alone cannot
support general natural-image quality claims. Native camera and production
HDR sequences remain a required corpus addition.

The corpus is fetched/generated rather than committed because raw media is
large, except for the small AI source PNG needed for exact reproducibility.
`scripts/fetch-corpus.sh` performs and verifies the exact conversion. Core
samples are immutable within a corpus version. Changing a sample, conversion,
or frame range requires a new version.

Procedural and AI samples expose controlled failure modes but cannot support
natural-image quality claims by themselves. Restricted or non-commercial
media may be used only in a separately reported diagnostic suite.

## Canonical input and dimensions

- Resolutions: 640x360, 1280x720, 1920x1080, and 3840x2160 progressive;
  aggregate results must also be grouped by resolution.
- Pixel format: planar 8-bit YUV 4:2:2.
- Matrix/range: BT.709, limited range.
- Frame rate: source rational rate, currently 24/1 for core media.
- Tile size: codec default (256x128) unless a tile experiment says otherwise.
- Qualities: 60, 75, 90, 95, and 100.
- Threads: 1 and `min(4, available logical CPUs)`.

Odd dimensions, grayscale, malformed streams, and alternate tile sizes remain
conformance tests rather than corpus performance rows. HDR and RGBA remain
capability-track rows until their native formats are implemented.

High-bit rows use planar YUV 4:2:2 with 10, 12, or 16 significant bits stored
in little-endian `u16` words. Results are grouped by bit depth and may not be
pooled with 8-bit results. Inputs must not be downshifted or tone-mapped.
Color primaries, transfer, matrix, and range are reported when known; the
current procedural supplement exercises numerical precision but does not
claim calibrated HDR colorimetry.

## Quality measurements

For every sample and quality:

- Y, Cb, and Cr PSNR from aggregate squared error, not averaged frame PSNR;
- maximum absolute sample error;
- mean per-frame luma 8x8 block SSIM;
- exact byte equality at quality 100.

PSNR and SSIM stabilization constants use the signaled peak
`2^bit_depth - 1`, not 255, for high-bit rows. MSE remains expressed in
native code-value units. Reports include bit depth so absolute MSE and maximum
error are not compared across depths without normalization.

Luma MS-SSIM and VMAF are required future additions before subjective quality
claims. Metrics must be reported per sample and as corpus arithmetic means;
compression and throughput summaries use both arithmetic and geometric means
to expose outliers.

Metric implementations are part of the protocol. Reports must pin window,
stride, scaling, border handling, pooling, model/weight version, color
conversion, and display/viewing model wherever applicable. A result from a
different implementation or parameter set is a different metric.

The metric feedback tiers are:

1. **Fast:** exact error and PSNR remain anchors. A top-left-anchored sample of
   every second 8x8 block in each axis may be reported as `sample2 block
   SSIM`; [EXP-0037](experiments/EXP-0037-sampled-block-ssim.md) measured
   maximum absolute error 0.000416, Spearman rho 0.999900, no material
   operating-point reversals, and 3.44x median metric speedup on the complete
   corpus. It is a rejection/screening diagnostic only. Every-fifth-block
   sampling failed its error gate and is not a standard metric.
2. **Focused:** MS-SSIM/VMAF and a texture-aware diagnostic such as DISTS are
   run on affected samples and rate points. Generated/fine-texture assets and
   UI/text assets remain separate groups because their failure modes conflict.
3. **Release/capability:** a temporal, color- and HDR-aware metric such as
   ColorVideoVDP is run for supported native HDR/video rows with its display
   model, transfer function, color space, frame rate, temporal padding,
   dependency versions, and CPU/GPU device recorded.

Decoded frames, color transforms, and scale pyramids should be shared among
slow metrics when this does not alter their normative inputs. Learned or
fused metrics are additional axes, not sole acceptance gates; no codec change
may be accepted by improving one learned score while exact error, PSNR, SSIM,
or a content group materially regresses.

The fully evaluated luma 8x8 block SSIM (block stride one) remains the
acceptance and release score. Any candidate that survives fast screening must
be rescored exactly before its experiment can be accepted.

## Compression measurements

Report encoded bytes, raw bytes, raw/encoded ratio, and bits per luma pixel.
For video, also report encoded stream data rate in decimal MB/s and bitrate in
decimal Mb/s, calculated from all frame/container/directory bytes divided by
the source playback duration. State the coding track and source frame rate.
This playback rate is distinct from encode/decode throughput. Quality 100 must
round-trip exactly. The initial all-intra target is:

- no core sample expands at quality 100;
- geometric-mean compression at least 2x at quality 100;
- geometric-mean compression at least 10x at the highest tested quality whose
  mean Y PSNR is at least 48 dB and mean luma block SSIM is at least 0.99.

These are engineering targets, not grounds for hiding per-sample failures.

## Speed and resource measurements

Build with the checked-in release profile. Record CPU model, logical CPU
count, RAM, OS, Rust version, commit, and whether the machine was otherwise
idle. For each matrix cell:

1. perform one unrecorded warm-up;
2. run five recorded trials in alternating baseline/candidate order;
3. report median encode and decode wall time, luma MP/s, and raw decimal MB/s;
4. separately report peak resident memory when the harness supports it.

Only one CPU-bound benchmark process may run at a time on the measurement
host. Independent benchmark matrices must execute serially; concurrent tool
dispatch invalidates their timing rows even when each matrix internally uses
balanced baseline/candidate order.

Quality metric calculation, source conversion, and file I/O are outside timed
encode/decode regions. A candidate fails the default speed gate if the
one-thread corpus median regresses by more than 5% without a documented,
accepted rate/quality tradeoff. Four-thread measurements are required but are
advisory on noisy shared VMs.

MP/s counts full-resolution luma pixels, independent of chroma subsampling.
For the canonical planar 8-bit YUV 4:2:2 input, an even-width frame contains
2 raw bytes per luma pixel, so 1 MP/s equals 2 decimal MB/s (approximately
1.907 MiB/s). Raw MB/s must be computed from actual plane byte counts so the
metric remains correct for odd widths and future pixel formats.

For planar 10/12/16-bit YUV 4:2:2 stored in `u16`, an even-width frame contains
4 raw bytes per luma pixel, so 1 MP/s equals 4 decimal MB/s. This storage
throughput is intentionally the same for all three depths; compressed bitrate
and bits per luma pixel expose their coding differences.

## Feedback tiers

Benchmark cost is proportional to confidence required:

1. **Fast iteration:** `scripts/benchmark-feedback.sh` runs five trials of four
   pinned spatial/temporal, one/four-thread cases. It targets a sub-minute
   runtime and reports per-case medians plus encode/decode geomeans. Use it
   while shaping an optimization; a result inside its measured 5% noise band
   is not evidence of improvement. Preserved-binary optimization comparisons
   use `scripts/benchmark-ab-feedback.sh`, an even trial count, warm-up of both
   binaries, and alternating execution order.
2. **Focused confirmation:** run every applicable corpus sample at the
   affected qualities, thread counts, and coding track, normally with three
   trials. Use this only after the fast tier shows a material effect.
3. **Release confirmation:** run the complete five-quality, one/four-thread,
   five-trial standard matrix plus access tests and conformance checks.

The fast tier is a rejection tool, not a substitute for the standard corpus.
Its cases and trial policy are versioned by
[EXP-0010](experiments/EXP-0010-fast-feedback-loop.md). Baseline and candidate
must use the same binary configuration, warm-up, corpus bytes, and host.
Direct performance comparisons should use separately preserved binaries and
`scripts/benchmark-ab-corpus.sh`. Its trial count must be even so each binary
runs first and second equally often within every sample/settings cell.
Native high-bit comparisons use `scripts/benchmark-ab-high-bit.sh`, the
checksummed high-bit manifest, and the same balanced-order rule. High-bit and
8-bit geomeans remain separate. Native high-bit single-frame-access
comparisons use `scripts/benchmark-access-high-bit-ab.sh`; it uses the same
target indices, warm-cache codec-only boundary, balanced order, and
per-bit-depth separation as the 8-bit access protocol.

Before preserving a candidate, run `cargo build --release` explicitly:
`cargo test --release` builds test executables but does not guarantee that the
standalone `target/release/fastvid` has been relinked. Record SHA-256 for both
preserved binaries and require distinct hashes whenever the candidate changes
code reachable by the CLI. Also record an exact-stream control hash. A stale
or identical candidate binary invalidates performance conclusions and must be
corrected by a new experiment record rather than silently reused.

## Exploration, exploitation, and the codec frontier

Fastvid maintains a portfolio rather than optimizing only the most recently
accepted implementation. The current 2--3 version registry is
[`FRONTIER.md`](FRONTIER.md), with slots for:

1. a balanced accepted line;
2. a compression-oriented line that accepts a documented speed cost; and
3. a throughput-oriented line that preserves rate and quality within its
   declared tolerances.

A slot may be vacant when no distinct candidate is non-dominated. Rejected or
strictly dominated versions are not promoted merely to fill a slot. Frontier
source is retained by an exact Git commit or source-archive hash; every entry
also records a distinct release-binary hash, exact-stream controls, experiment
evidence, and benchmark artifact hashes.

In every rolling group of six optimization experiments, at least two must be
**exploration** of a materially different technique family and at least two
must be **exploitation** of a measured frontier bottleneck. The remaining two
follow the strongest evidence. Predictor families, residual representations,
entropy formats, color transforms, temporal structures, and tile structures
count as distinct exploration families. Parameter tweaks and additional
implementations of the same hot loop count as exploitation.

Exploration starts with a cheap oracle, exact byte model, or microbenchmark.
It advances to focused confirmation only after a predeclared gate passes.
This keeps the search broad without spending full-corpus CPU time on weak
ideas. Exploitation uses profiles and preserved-binary A/B tests against the
specific frontier slot it intends to improve.

At matched settings, a candidate is dominated only when another version is no
worse in complete encoded bytes, quality, encode speed, decode speed, and
single-frame access outside the standard measurement tolerance, and
materially better in at least one. The default comparison tolerances are 1%
for encoded bytes, 5% for timing, and exact q100 reconstruction. Deliberate
quality/rate tradeoffs must declare their operating point and cannot be
compared as if controls were equivalent.

## Single-frame access measurements

Single-frame access is a separate editing/random-access result, not a
sequential throughput result. For each standard 24-frame video, quality,
thread count, and coding track, request targets 0, 1, 6, 11, 12, 13, 18, and
23 from a fresh decoder state. For short-GOP video, begin at the nearest
preceding keyframe, decode dependencies through the target, and discard
preroll output. For all-intra, decode only the requested frame.

The initial benchmark is warm-cache and codec-only: sequence encoding, index
lookup, source/container I/O, and quality metrics remain outside the timed
region. Record per target:

- target and keyframe indices;
- dependency/preroll frames and total decoded frames;
- compressed bytes read from the keyframe through the target;
- target access wall time;
- useful-target MP/s and raw MB/s;
- actual decoded-work MP/s;
- access amplification in decoded frames per requested frame.

Report median, p95, and worst access latency plus the target responsible for
the worst result. Preserve all per-target rows. GOP 1 and GOP 12 must use the
same requested target indices. State the maximum dependency depth and never
mix these access results with sequential decode MP/s. A future indexed
sequence container must add cold-cache lookup and I/O latency separately.

## Rate-distortion and acceptance

Preserve per-sample rows. Plot or tabulate quality against bits per pixel,
PSNR, and SSIM. Bjøntegaard delta-rate is a future summary once at least four
overlapping stable rate points and the comparison implementation exist.

An experiment may be accepted when:

- all conformance, malformed-input, strict lint, release, and Lean checks pass;
- quality 100 remains exact;
- no unexplained core-corpus regression exceeds 5%;
- its hypothesis-specific gate passes;
- the experiment record includes full commands, host details, and results.

Synthetic/demo fixtures remain useful diagnostics but cannot support general
compression or quality claims.

## External codec comparisons

External comparisons use the same source bytes, sample precision, coding
track, tile geometry where configurable, and thread count. Controls such as
Fastvid quality and another codec's QP are not assumed equivalent. Sweep the
reference codec, then select the measured point with minimum Y-PSNR distance;
report that distance and both SSIM values. Do not present interpolated timing
or bitrate as a measured row.

The standard OpenAPV diagnostic uses the high-bit corpus-v2
`high-precision-motion-10` sequence at 1280x720, 24 frames, and 24 fps:

- Fastvid GOP 1 versus OpenAPV's intra-only coding;
- explicit 256x128 tiles and 1/4 threads;
- OpenAPV `medium` (upstream default) and `fastest` reported separately;
- one warm-up and five serial trials;
- each codec application's internal codec clocks, with file I/O excluded;
- common Fastvid PSNR, SSIM, and maximum-error measurement.

Fastvid q100 may be called matched only when the other reconstruction is also
exact. Version, compiler, architecture dispatch, command lines, encoded bytes,
bits/luma-pixel, stream MB/s and Mb/s, and all timing/quality rows are retained.
OpenAPV implementation details supporting this protocol are recorded in
[research 0015](research/0015-openapv-matched-comparison.md).
