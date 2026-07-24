# EXP-0057 — Automated Pareto frontier

Status: **ACCEPTED**

## Classification

**Evaluation infrastructure** — make the existing multi-objective codec
registry machine-readable and measure every preserved version under one
balanced protocol.

## Hypothesis

A hash-validated, multi-version feedback run can automatically expose the
compression/encode/decode Pareto relationships without comparing timings
from different dates or turning the README into a changelog.

## Modification

- Add `frontier.json` as the machine-readable companion to `FRONTIER.md`.
- Represent balanced, practical-compression, maximum-compression, and speed
  roles explicitly; a role may be vacant and does not count as a preserved
  candidate.
- Add a serial multi-version benchmark with warm-up, six trials, and cyclic
  execution order so every active version appears twice in every position.
- Add a standard-library-only graph tool that validates the result matrix,
  computes per-case medians/geometric means, identifies non-dominated points,
  emits encode/decode SVG panels, and writes a compact durable TSV summary.

## Test

- Reject missing binaries, hash mismatches, unknown/vacant measured slots,
  incomplete trials, and size instability across trials.
- Unit-test Pareto rejection with synthetic dominated points.
- Run all preserved binaries on the four feedback cases.
- Confirm that the graph and summary regenerate deterministically from the
  retained raw TSV.

## Gate

- Every active binary hash must match the registry.
- Every slot/case must contain trials 1 through 6.
- The summary must include the vacant speed slot without plotting it.
- Re-running the graph tool over identical input must reproduce byte-identical
  summary and SVG files.
- The human and machine-readable registries must agree.

## Result

The six-trial run validated all three preserved binary hashes, warmed each
version for every case, and recorded 72 rows: four cases by three versions by
six trials. Cyclic ordering put every version in each execution position
twice per case. The raw artifact is
`artifacts/frontier-fast-feedback.tsv`
(`4cccd8ee98c0a63b256d06ab182fcc48d8f61c7910f4077e242fe91f64cabfa2`).

Per-case medians followed by geometric means produced:

| Slot | Compression | Encode | Decode | Playback bitrate |
|---|---:|---:|---:|---:|
| Balanced | 14.504x | 95.708 MP/s | 101.256 MP/s | 63.394 Mb/s |
| Practical compression | 24.548x | 28.788 MP/s | 132.382 MP/s | 37.455 Mb/s |
| Maximum compression | 33.613x | 24.143 MP/s | 94.824 MP/s | 27.354 Mb/s |
| Speed | vacant | vacant | vacant | vacant |

The result demonstrates why separate encode and decode panels matter.
Balanced is the encode-throughput point, practical compression is the
decode-throughput point, and maximum compression is the size point. No
distinct fourth version currently qualifies for the speed role.

The graph tool rejected a synthetic dominated point in its self-test,
validated trials 1 through 6 and stable encoded size for every slot/case, and
included the vacant speed role in the summary without plotting it. Two
successive generations were byte-identical:

- `benchmarks/frontier.svg`
  (`e098314daab684170c0b57e9829e460f1b612982fb654b26c8920d2c8499c20c`);
- `benchmarks/frontier-summary.tsv`
  (`d5f67d031d766d5e45a6d4fb7732d7c98aba4c2bb27b779f7b111054980c646b`).

During the first attempted run, hash validation caught that the ignored
maximum-compression artifact had been overwritten by a later analyzer build.
The benchmark did not start. Restoring the preserved source-state binary
recovered its registered
`dda826459cfa9cb017b751749d2b780419b18cc1a2ff9ff309492ea8b4df61da`
hash, after which the measurement passed. This is direct evidence that the
guard is useful rather than decorative.

## Decision

Accept the automated registry and graph workflow. Keep the speed role
explicitly vacant until a distinct candidate is non-dominated; do not
duplicate the balanced binary merely to fill the table. Use the generated
graph as a current-state screening view and immutable experiments as the
detailed historical record.
