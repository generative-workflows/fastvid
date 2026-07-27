# Frontier benchmark summaries

This directory holds small, durable summaries for cross-version comparisons.
Raw repeated measurements remain under `artifacts/` and are referenced by
SHA-256 from immutable experiment records.

`frontier.json` is the machine-readable registry. A comparable frontier run
and graph are produced with:

```sh
scripts/benchmark-frontier.sh artifacts/frontier-fast-feedback.tsv
scripts/graph-frontier.py \
  artifacts/frontier-fast-feedback.tsv \
  benchmarks/frontier.svg \
  --summary benchmarks/frontier-summary.tsv
```

The harness validates every preserved binary hash, warms every version, runs
one CPU-bound process at a time, and rotates execution order so each of the
three active versions occupies every position twice in six trials. The graph
uses per-case medians and then geometric means. It is a fast screening view,
not a replacement for the complete corpus, quality, memory, or access
matrices.

The separate matched OpenAPV diagnostic is produced with:

```sh
scripts/benchmark-openapv-frontier.sh \
  artifacts/openapv-frontier-current.tsv \
  /tmp/openapv-cmake-build artifacts/corpus-v2 6 frontier.json
scripts/graph-openapv-frontier.py \
  artifacts/openapv-frontier-current.tsv \
  benchmarks/openapv-frontier.svg \
  --summary benchmarks/openapv-frontier-summary.tsv \
  --trials 6
```

[`openapv-frontier.svg`](openapv-frontier.svg) and
[`openapv-frontier-summary.tsv`](openapv-frontier-summary.tsv) use one
checksummed native-10-bit sequence, all-intra coding, matched tile geometry,
and measured PSNR selection. They do not share coordinates with the four-case
8-bit graph. The SVG shows one-thread q90-neighborhood results; the TSV also
retains four-thread rows and the non-exact OpenAPV QP0 high-fidelity boundary.

OpenAPV is a pinned external target. By default the harness validates and
reuses the checksummed six-trial OpenAPV rows named in `frontier.json`, then
remeasures only the current Fastvid slots. Pass `--refresh` as the sixth
argument only when the OpenAPV binary, corpus, controls, or benchmark machine
changes. This keeps routine promotion feedback focused on code that changed.

## CPU baseline for CUDA

[`v5-cpu-baseline.md`](v5-cpu-baseline.md) is the durable version-5
rate/quality/thread-scaling reference collected before CUDA migration. It
uses the checksummed native high-bit supplement and keeps deterministic
rate/quality rows separate from five-trial timing rows.

Durable per-sample and aggregate data:

- [`v5-cpu-baseline-quality.tsv`](v5-cpu-baseline-quality.tsv);
- [`v5-cpu-baseline-quality-summary.tsv`](v5-cpu-baseline-quality-summary.tsv);
- [`v5-cpu-baseline-speed.tsv`](v5-cpu-baseline-speed.tsv);
- [`v5-cpu-baseline-speed-summary.tsv`](v5-cpu-baseline-speed-summary.tsv).

The raw measurements and host record remain under `artifacts/` with hashes in
[EXP-0135](../experiments/EXP-0135-cpu-gpu-baseline.md). This baseline is
separate from the preserved CPU frontier and does not alter the root README.

## CUDA decoder baseline

[`v5-cuda-decode-baseline.md`](v5-cuda-decode-baseline.md) records the first
byte-exact PyTorch C++/CUDA v5 decoder on a real-world 4K frame. Machine-
readable rows are in [`v5-cuda-decode-baseline.tsv`](v5-cuda-decode-baseline.tsv).

## CUDA feedback loop

Before optimizing GPU encoding, collect a joint real-footage baseline with:

```sh
scripts/benchmark-cuda-feedback.sh \
  target/release/fastvid artifacts/corpus-v3 artifacts/cuda-feedback 5 quick
```

The quick scope converts eight pinned first frames to 10-bit 4:2:2. It spans
lossless-source 1080p animation and TIFF camera material, a procedural 1080p
edge control, rendered 2K/4K controls, and real-world crowd/animal 4K footage.
Use `full` instead of `quick` to cover the first frame of every codec-track
sample in `corpus/manifest.json`. Both scopes record repeated Rust encode
timing, complete rate/quality/XPSNR controls, CUDA DRAM/VRAM decode timing,
environment data, and hashes. Q90 is the practical point and q100 is the
exactness control.

The standard v3 derivatives are 8-bit, so conversion to 10-bit preserves
their values but does not create missing source precision. WebM-derived rows
are representative of decoded real-world structure, not pristine camera
acquisition. The TIFF/PNG-derived, procedural, and rendered rows prevent those
clips from being the only evidence.

Generate the aggregate target-gap report without discarding raw rows:

```sh
scripts/summarize-cuda-feedback.py \
  artifacts/cuda-feedback \
  benchmarks/v5-cuda-feedback.md \
  benchmarks/v5-cuda-feedback-summary.tsv
```
Rust-oracle encode, CUDA encode, and CUDA decode remain explicit separate
panels; kernel-only timing cannot be substituted for complete-call
measurement. Every measured CUDA stream is compared byte-for-byte with the
Rust stream produced in the same case.

The current 24-sample report, including its separate 15-sample 1080p slice, is
[`v5-cuda-feedback-encoder.md`](v5-cuda-feedback-encoder.md); machine-readable
aggregates are in
[`v5-cuda-feedback-encoder-summary.tsv`](v5-cuda-feedback-encoder-summary.tsv).

[`v5-cuda-encode-progress.md`](v5-cuda-encode-progress.md) records the
byte-identical CUDA encoder baseline and each accepted optimization using the
same real-world 4K control. Its TSV companion preserves machine-readable stage
and complete-call measurements.
