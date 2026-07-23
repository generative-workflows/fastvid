# Fastvid evaluation methodology

Version: 1 (2026-07-23)

This document defines the standard target used to evaluate Fastvid changes.
Experiments may add diagnostics or deliberately diverge, but an optimization
is not a general improvement unless it is measured against this methodology.
The design is grounded in [research 0004](research/0004-codec-evaluation.md)
and [research 0006](research/0006-standard-evaluation-methodology.md).

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
[`corpus/manifest.json`](corpus/manifest.json). It contains:

- six 1920x1080 still frames spanning fine organic detail, smooth gradients,
  text, dark sparse content, high-frequency lines, and dense geometry;
- three 24-frame 1920x1080 clips spanning fine-detail motion, foliage motion,
  and dense articulated motion;
- lossless PNG sources from *Big Buck Bunny* and *Elephants Dream*, with
  explicit CC BY licenses and upstream SHA-256 verification;
- canonical BT.709 limited-range planar YUV422p8 derivatives with committed
  SHA-256 values.

The corpus is fetched rather than committed because raw media is large.
`scripts/fetch-corpus.sh` performs and verifies the exact conversion. Core
corpus samples are immutable within a corpus version. Changing a sample,
conversion, or frame range requires a new version.

The core is intentionally small enough for every meaningful experiment. It
currently overrepresents rendered content. The **extended corpus** must add
permissively licensed camera footage, screen capture, deterministic noise,
film grain, animation, scene cuts, and low/high-motion clips before broad
release claims. Restricted or non-commercial media may be used only in a
separately reported diagnostic suite, never the standard corpus.

## Canonical input and dimensions

- Resolution: 1920x1080 progressive.
- Pixel format: planar 8-bit YUV 4:2:2.
- Matrix/range: BT.709, limited range.
- Frame rate: source rational rate, currently 24/1 for core media.
- Tile size: codec default (256x128) unless a tile experiment says otherwise.
- Qualities: 60, 75, 90, 95, and 100.
- Threads: 1 and `min(4, available logical CPUs)`.

Odd dimensions, grayscale, malformed streams, and alternate tile sizes remain
conformance tests rather than corpus performance rows.

## Quality measurements

For every sample and quality:

- Y, Cb, and Cr PSNR from aggregate squared error, not averaged frame PSNR;
- maximum absolute sample error;
- mean per-frame luma 8x8 block SSIM;
- exact byte equality at quality 100.

Luma MS-SSIM and VMAF are required future additions before subjective quality
claims. Metrics must be reported per sample and as corpus arithmetic means;
compression and throughput summaries use both arithmetic and geometric means
to expose outliers.

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
