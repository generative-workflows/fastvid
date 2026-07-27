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
