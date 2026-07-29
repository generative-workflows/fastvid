# EXP-0157 — Bounded palette order-0 fallback

Status: **REJECTED**

## Hypothesis

A deterministic fixed-width palette for shards containing 2–16 distinct
folded residuals will capture a useful GPU-native subset of EXP-0152's
order-0 opportunity, reducing encoded size without adding enough analyzer or
decoder work to violate the strict latency gates.

## Modification

Add shard entropy mode 19 with a sorted 32-bit palette and fixed-width palette
indices. Discover at most 17 symbols through a bounded shared-memory hash table,
select the mode only when its exact charged body is smaller than zero-run,
Rice, and block-pack, and decode indices in parallel. The canonical evaluator
was unchanged.

## Test

Run the frozen evaluator with 5 warm-ups and 20 repetitions on:

- `/tmp/fastvid-perf/manifest.json` at q95, producing
  `/tmp/perf-palette16-q95.json`;
- `/tmp/fastvid-rejection/manifest.json` at q90, producing
  `/tmp/palette16-matrix-q90.json`.

Baseline artifacts were `/tmp/perf-rice-cache-q95.json` and
`/tmp/depth-preserving-validated.json`.

## Result

The q95 RGB10 control remained exactly 2,905,962 bytes, proving no palette
selection. Median encode latency increased from 0.798448 to 0.805712 ms. The
candidate run measured decode at 0.504544 ms versus 0.498224 ms and failed the
strict 0.5 ms gate, despite identical decode bytes and path.

Across all nine matrix samples, every encoded size was byte-identical to the
1,424,526-byte baseline. Quality passed unchanged (minimum SSIMULACRA2
93.179863, maximum Butteraugli 0.795349), but the new mode was never smaller.

## Decision

Rejected and reverted. EXP-0152's benefit comes from probability skew within
larger alphabets, not merely small distinct-symbol counts. A subsequent
order-0 implementation must use normalized frequencies (for example rANS)
and must avoid charging all shards with material discovery overhead.

## References

- [EXP-0152](EXP-0152-v5-full-frame-order0-model.md)
- [Research 0024](../research/0024-finite-block-ans-entropy-models.md)
- [Research 0030](../research/0030-entropy-decode-consumer-fusion.md)
