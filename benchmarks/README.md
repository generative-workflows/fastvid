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
one CPU-bound process at a time, and rotates execution order so each active
version occupies every position equally. The graph uses per-case medians and
then geometric means. It is a fast screening view, not a replacement for the
complete corpus, quality, memory, or access matrices.
