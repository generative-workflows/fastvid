# EXP-0158 — CUDA four-lane shard order-0 rANS

Status: **PENDING FULL CORPUS**

Implementation revision: `149a40b`

## Hypothesis

A bounded normalized-frequency order-0 fallback with four independent rANS
states will recover at least 2% complete bytes on the required matrix while
retaining the strict RGB 1080p latency gates. Caching exact renormalization
bytes during analysis will avoid paying the state transition twice.

## Modification

Add bounded-shard entropy mode 19:

- histogram folded values 0–510 and retain alphabets of at most 255 symbols;
- normalize one deterministic 4096-slot table using cumulative quantiles;
- encode four independent byte-renormalized rANS lanes;
- cache the sparse table, lane bytes, and final states during exact analysis;
- assemble cached bytes in the existing shard emission kernel;
- parse canonical varints, validate sorted symbols/frequencies/states and exact
  body consumption, and decode four lanes through an 8-bit shared lookup table;
- fuse legacy and order-0 decode dispatch into one all-shard kernel;
- use order-0-first selection for RGB to satisfy its latency gate, and exact
  legacy/order-0 byte selection for YUV422 and gray.

Quality, prediction, quantization, and `scripts/evaluate.py` are unchanged.
Values outside the bounded alphabet, including difficult q100 shards, retain
the existing entropy modes.

## Canonical evaluation

All commands used the frozen evaluator, FFVShip 5.0.0-a, five warm-ups, and 20
repetitions.

- RGB10 1920x1080 q95 control:
  `/tmp/perf-rans4-hybrid2-q95.json`
- nine-sample required format/depth q90 matrix:
  `/tmp/rans4-hybrid2-matrix-q90.json`
- the same matrix at q100:
  `/tmp/rans4-hybrid2-matrix-q100.json`

Baselines were `/tmp/perf-rice-cache-q95.json` and
`/tmp/depth-preserving-validated.json`.

## Result

The RGB10 q95 first frame fell from 2,905,962 to 2,616,088 bytes (-9.98%).
The final canonical run measured 0.871856 ms median encode and 0.435456 ms
median decode, clearing the strict 1.0/0.5 ms gates. SSIMULACRA2 remained
96.772240 and Butteraugli remained 0.778911.

The q90 matrix fell from 1,424,526 to 1,393,259 bytes (-2.195%) and passed all
nine samples, with minimum SSIMULACRA2 93.179863 and maximum Butteraugli
0.795349. The q100 matrix passed all nine samples with minimum SSIMULACRA2
99.811905 and Butteraugli 0.0.

Malformed CUDA streams with an invalid table log, zero alphabet, unknown mode,
or truncation were rejected. `pytest -q` passed five evaluator tests; four
legacy CUDA tests could not start because their removed Rust oracle binary
`target/release/fastvid` is absent. Those failures did not execute candidate
code.

## Decision

Retain as a promising candidate, but do not mark accepted yet. The repository
still has no checked-in canonical full manifest/corpus satisfying the current
INSTRUCTIONS.md coverage, so the mandatory unchanged full tier and 4K x24
throughput gates cannot yet be established. A full-corpus artifact is required
before changing this status to accepted.

## References

- [EXP-0053](EXP-0053-finite-block-order0-model.md)
- [EXP-0068](EXP-0068-four-state-rans.md)
- [EXP-0152](EXP-0152-v5-full-frame-order0-model.md)
- [Research 0024](../research/0024-finite-block-ans-entropy-models.md)
- [Research 0030](../research/0030-entropy-decode-consumer-fusion.md)
