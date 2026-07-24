# Fastvid codec frontier

This is a current-state registry, not an experiment log or changelog. It keeps
at most three distinct non-dominated codec versions across four named roles.
A role remains explicitly vacant when no additional version earns it.
Completed evidence remains in immutable experiment records. The
machine-readable companion is [`frontier.json`](frontier.json).

## Frontier slots

| Slot | Source | Binary SHA-256 | Stream compatibility | Evidence | State |
|---|---|---|---|---|---|
| Balanced | `156054c` | `06ef3278e9055f3c53c94cf964f4a7bf785453b696e0df262dec9161b45c6ab8` | v0 8-bit / v1 high-bit | [EXP-0045](experiments/EXP-0045-rolling-8bit-reconstruction.md) | Preserved |
| Practical compression | `4ad0318` | `1235c7e82cf34fdddf5c341a5c17d265687368092174d175db709f22b17131c9` | v2 encode; v0/v1/v2 decode | [EXP-0052](experiments/EXP-0052-16bit-temporal-decode-guard.md) | Preserved |
| Maximum compression | `84a3be1` | `dda826459cfa9cb017b751749d2b780419b18cc1a2ff9ff309492ea8b4df61da` | 8-bit v3 / high-bit v2 encode; legacy decode | [EXP-0055](experiments/EXP-0055-modeled-rans-selector.md) | Preserved |
| Speed | — | — | — | No distinct candidate | Vacant |

All three active sources are retained in Git. The balanced version currently
has the highest measured encode throughput, but duplicating it in the speed
role would not add a frontier point. The speed role is reserved for a
distinct candidate that earns its place.

## Automated view

The current comparable fast-feedback view is
[`benchmarks/frontier.svg`](benchmarks/frontier.svg), with exact aggregates in
[`benchmarks/frontier-summary.tsv`](benchmarks/frontier-summary.tsv).
`scripts/benchmark-frontier.sh` validates every binary hash and records a
balanced multi-version run; `scripts/graph-frontier.py` validates that matrix,
finds non-dominated points, and regenerates both files. This is a screening
view, not a substitute for complete corpus, access, quality, or memory
evidence.

## Active technology tree

```text
                              accepted balanced line
                                      |
                    rolling reconstruction / exact Rice
                       /                              \
       compatible predictor oracle              throughput exploration
                |                              scheduler/SIMD/cache work
        version-2 tile modes
          /             \
 practical guard     maximum compression
 16-bit temporal     8-bit tile-local rANS
```

## Promotion and retirement

A candidate enters a slot only after its immutable experiment record contains:

- a source commit or source archive hash;
- a distinct release binary SHA-256;
- exact-stream controls and quality evidence;
- complete encoded bytes, encode/decode throughput, and access behavior for
  the candidate's declared scope; and
- the artifact hashes needed to reproduce its position.

At identical input, quality, coding track, and thread count, version A
dominates B only when A is no worse outside the measurement tolerance in
encoded bytes, quality, encode speed, decode speed, and access cost, and is
materially better in at least one. The standard tolerances are exact quality
invariance at q100, 1% for encoded bytes, and 5% for timing. A deliberate
rate/quality tradeoff remains non-dominated when it occupies a declared slot.

When a new version dominates a slot, this file replaces the row; the old
experiment remains immutable and Git retains its source. At most three
versions are active so confirmation cost remains bounded.

## Preserved artifacts

- `artifacts/frontier/fastvid-balanced-exp0045`
  (`06ef3278e9055f3c53c94cf964f4a7bf785453b696e0df262dec9161b45c6ab8`);
- `artifacts/frontier/fastvid-compression-exp0052`
  (`1235c7e82cf34fdddf5c341a5c17d265687368092174d175db709f22b17131c9`);
- `artifacts/frontier/fastvid-rans-exp0055`
  (`dda826459cfa9cb017b751749d2b780419b18cc1a2ff9ff309492ea8b4df61da`).
