# Fastvid codec frontier

This is a current-state registry, not an experiment log or changelog. It keeps
two or three distinct codec versions that best represent the measured
technology frontier.
Completed evidence remains in immutable experiment records. The
machine-readable companion is [`frontier.json`](frontier.json).

## Frontier slots

| Slot | Source | Binary SHA-256 | Stream compatibility | Evidence | State |
|---|---|---|---|---|---|
| Practical compression | `4ad0318` | `1235c7e82cf34fdddf5c341a5c17d265687368092174d175db709f22b17131c9` | v2 encode; v0/v1/v2 decode | [EXP-0052](experiments/EXP-0052-16bit-temporal-decode-guard.md) | Preserved |
| Maximum compression | `84a3be1` | `dda826459cfa9cb017b751749d2b780419b18cc1a2ff9ff309492ea8b4df61da` | 8-bit v3 / high-bit v2 encode; legacy decode | [EXP-0055](experiments/EXP-0055-modeled-rans-selector.md) | Preserved |
| Speed | `4ad0318` + `exp0060-speed.patch` | `f8e6bb69d7cf52b4531210e7423ec75a5626549ac1bacc964c1e123ca2bde8f7` | v2 encode; v0/v1/v2 decode | [EXP-0060](experiments/EXP-0060-fixed-gradient-speed-tier.md) | Preserved |

The speed source is reproduced by applying
`artifacts/frontier/exp0060-speed.patch` to Git commit `4ad0318`; the other
two active base sources are retained directly in Git. The speed tier uses fixed
clamp-gradient intra prediction and frame-gated temporal prediction, trading
some spatial compression for materially higher encode, decode, and
single-frame-access throughput.

The former balanced snapshot (`156054c`, binary `06ef3278…6ab8`) remains a
reproducible historical reference under EXP-0045. EXP-0061 retired it from
routine active measurement because speed gains 26.07% encode and 41.23%
decode throughput for a 7.93% compression-ratio cost.

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
       compatible predictor oracle              fixed-gradient speed tier
                |                              direct intra/temporal paths
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

- `artifacts/frontier/fastvid-compression-exp0052`
  (`1235c7e82cf34fdddf5c341a5c17d265687368092174d175db709f22b17131c9`);
- `artifacts/frontier/fastvid-rans-exp0055`
  (`dda826459cfa9cb017b751749d2b780419b18cc1a2ff9ff309492ea8b4df61da`);
- `artifacts/frontier/fastvid-speed-exp0060`
  (`f8e6bb69d7cf52b4531210e7423ec75a5626549ac1bacc964c1e123ca2bde8f7`),
  reproduced by `artifacts/frontier/exp0060-speed.patch`.

Historical reference:

- `artifacts/frontier/fastvid-balanced-exp0045`
  (`06ef3278e9055f3c53c94cf964f4a7bf785453b696e0df262dec9161b45c6ab8`).
