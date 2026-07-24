# Fastvid codec frontier

This is a current-state registry, not an experiment log or changelog. It keeps
the two or three non-dominated codec versions that best represent the active
technology tree. Completed evidence remains in immutable experiment records.

## Frontier slots

| Slot | Source commit | Binary SHA-256 | Stream compatibility | Evidence | State |
|---|---|---|---|---|---|
| Balanced | `156054c` | `06ef3278e9055f3c53c94cf964f4a7bf785453b696e0df262dec9161b45c6ab8` | v0 8-bit / v1 high-bit | [EXP-0045](experiments/EXP-0045-rolling-8bit-reconstruction.md) | Preserved |
| Compression | — | — | — | Predictor-bounded mapping and compatible predictor selection are being screened by [EXP-0046](experiments/EXP-0046-predictor-bounded-residual-model.md) | Vacant |
| Throughput | — | — | — | No distinct non-dominated version currently survives the evidence gates | Vacant |

The balanced source is retained in Git. Its preserved binary hash and exact
stream controls are recorded in EXP-0045; a binary may be regenerated from
the source commit when needed. Vacant slots are explicit: a rejected or
strictly dominated candidate is not kept merely to fill the table.

## Active technology tree

```text
                              accepted balanced line
                                      |
                    rolling reconstruction / exact Rice
                       /                              \
       compression exploration                 throughput exploration
       bounded residual symbols                scheduler/SIMD/cache work
                |                                      |
       compatible predictor oracle             only after space pass
                |
       table-charged entropy model
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
