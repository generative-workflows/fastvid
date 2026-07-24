# EXP-0056 — Causal context-conditioned order-0 model

Status: **REJECTED**

## Classification

**Exploration of a new residual-probability family** — condition entropy on a
causal local-scale signal rather than changing the rANS kernel.

## Hypothesis

Splitting each tile's folded residuals by the magnitude of the preceding
decoded folded residual will reduce complete modeled bytes by at least 3%
relative to EXP-0055's single order-0 table on natural, synthetic/UI, and
temporal content after charging every table, state, and substream control.

## Modification

- Extend the read-only entropy analyzer to retain normative folded residual
  order.
- Model two contexts (previous residual zero/nonzero) and three contexts
  (zero, `1..threshold`, above threshold) for thresholds 1, 3, 7, and 15.
- Normalize each nonempty context independently at table logs 8 through 12.
- Charge a one-byte context mode, one-byte threshold where applicable,
  varint substream lengths, one sparse table and four-byte state per nonempty
  context, and independent payload rounding.
- Do not change the codec or bitstream in this experiment.

## Test

- Prove by unit tests that contexts partition every symbol exactly and that
  empty/singleton contexts have deterministic costs.
- Run a fast screen on the four standard feedback cases.
- If no context clears 3% in at least three cases, reject before the complete
  corpus.
- Otherwise run the complete 8-bit q90/q100 entropy-model matrix and report
  category, prediction mode, plane, resolution, and winning-threshold
  distributions.

## Gate

- At least 3% complete modeled saving against tile-local order-0 bytes on
  each of natural, synthetic/UI, and temporal aggregate groups.
- No group may rely on uncharged tables, state, padding, or substream framing.
- Every tile retains independent access; no neighboring tile or future symbol
  may define its context.

## References

- [Research 0025](../research/0025-context-conditioned-residual-entropy.md)
- [EXP-0053](EXP-0053-finite-block-order0-model.md)
- [EXP-0055](EXP-0055-modeled-rans-selector.md)

## Result

The four-case q90/q100 screen cleared its advancement rule:

- camera 1080p: 0.43% fewer modeled order-0 bytes;
- hard cuts GOP 12: 5.01% fewer;
- 4K resolution grid: 10.97% fewer;
- UI motion GOP 12: 8.03% fewer;
- aggregate: 4.66% fewer.

The exact per-tile fallback selected a context model on 1,109 of 4,410 tiles.
The screen artifact is
`artifacts/exp0056-context-order0-screening.tsv`
(`73f1e8dc4caf389daf8d3b826ee133284bc36a7f0fc4fc9c856c78fa65cba132`).

The complete 18-sample q90/q100 matrix contained 62,064 tiles and 312 frames.
After charging a complete normalized table and four-byte state per nonempty
context, mode/threshold bytes, a varint length for every context substream,
independent payload rounding, the unchanged frame header, and every directory
entry, the exact fallback measured:

- single-table order-0: 238,509,505 bytes;
- causal-context oracle: 234,107,355 bytes;
- change: 4,402,150 fewer bytes, or 1.846%;
- winning tiles: 30,300.

Group results were:

- natural cinema: -3.545%;
- camera (including noisy video): -0.325%;
- AI-generated: -1.904%;
- synthetic/UI (including hard cuts): -0.753%;
- spatial-predicted tiles: -1.976%;
- temporal-predicted tiles: -1.456%;
- luma: -2.319%; Cb: -1.147%; Cr: -1.146%.

The result is highly content-dependent. The 4K grid saved 10.75%, monochrome
lines 7.13%, dark content 7.83%, and UI motion 6.87%, but noisy camera saved
0.25%, scene cuts 0.04%, and chroma edges nothing. Winning choices were
11,322 two-context tiles and 18,978 three-context tiles distributed across
all four tested thresholds.

The complete artifact is
`artifacts/exp0056-context-order0-model.tsv`
(`1b3ab5a1d5d0616de933c1266aa7194264d0d4fa42593a434f1e78858079ee02`).
Unit tests verified deterministic context partitioning and explicitly charged
empty-substream lengths plus singleton table/state costs. The codec and
bitstream were not changed.

## Decision

Reject the previous-residual-magnitude context as a general format addition.
It misses the predeclared 3% gate for camera and synthetic/UI aggregates and
does not justify multiple rANS states or added decode dispatch for a 1.85%
complete-corpus oracle.

The sample-level wins support a distinct future experiment using causal
spatial gradient or predictor-error magnitude, closer to the literature,
rather than relaxing this experiment's gate or adding content-specific
format heuristics.
