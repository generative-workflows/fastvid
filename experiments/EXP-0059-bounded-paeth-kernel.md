# EXP-0059 — Bounded Paeth kernel

Status: **REJECTED**

## Classification

**Speed-frontier exploitation** — replace a measured arithmetic hotspot while
preserving every prediction, reconstructed sample, and encoded byte.

## Hypothesis

The threshold/min/max Paeth decision documented in Research 0026 will reduce
balanced encode and spatial decode time by at least 2% on the focused
noisy-camera workload, without changing output bytes. It should also avoid a
regression greater than 1% on the automated frontier matrix.

## Modification

Replace the three-distance Paeth implementation in both 8-bit and high-bit
codecs with the equivalent bounded-threshold decision:

```text
threshold = 3*upper_left - (left + above)
lo = min(left, above)
hi = max(left, above)
candidate = lo if hi <= threshold else upper_left
result = hi if threshold <= lo else candidate
```

Retain safe Rust and `i32` arithmetic. Add exhaustive equivalence coverage for
all 16,777,216 8-bit input triples and edge/tie coverage over the 16-bit
domain.

## Test

Fast feedback:

1. run unit tests, including exhaustive equivalence;
2. compare candidate and exact preserved baseline bytes;
3. alternate baseline/candidate across at least six focused one-thread trials;
4. report encode/decode medians and spread.

Slow confirmation only if the focused gate passes:

1. run the standard automated four-case frontier matrix;
2. regenerate the frontier TSV and SVG;
3. promote a distinct speed binary only if the candidate is non-dominated.

## Gate

- identical encoded bytes and decoded metrics;
- at least 2% focused median encode or decode improvement;
- no opposite-direction focused regression above 1%;
- no standard-matrix geomean regression above 1%.

## References

- [Research 0026](../research/0026-paeth-data-dependency-kernel.md)
- [EXP-0058](EXP-0058-frontier-speed-profile.md)
- [EXP-0057](EXP-0057-automated-pareto-frontier.md)

## Results

The exhaustive test covered all 16,777,216 8-bit triples, and representative
high-bit extrema and tie cases covered 10/12/16-bit arithmetic. All comparisons
matched the original predictor. Both A/B runs used one warm-up per binary and
six alternating trials.

| Source line | Variant | Encoded bytes | Encode MP/s | Decode MP/s |
|---|---|---:|---:|---:|
| current maximum compression (`84a3be1`) | baseline | 29,518,163 | 13.4715 | 42.7570 |
| current maximum compression | bounded | 29,518,163 | 12.9570 | 43.2950 |
| balanced (`156054c`) | baseline | 32,630,454 | 38.6390 | 54.6040 |
| balanced | bounded | 32,630,454 | 34.6865 | 58.1110 |

Relative to baseline, the bounded form changed:

- current: encode -3.82%, decode +1.26%;
- balanced: encode -10.23%, decode +6.42%.

Encoded bytes, PSNR, block SSIM, maximum error, and tile-mode counts were
identical in every trial. The result is therefore an implementation tradeoff,
not a semantic difference. On this Rust/LLVM build the original absolute-value
form is substantially better for encode even though the bounded form improves
decode.

Raw artifacts:

- `artifacts/exp0059-current-ab.tsv`
  (`14e84b04593633ac54bb9c3617fc2cc3442a75c43b440b744d064789ca5186be`);
- `artifacts/exp0059-balanced-ab.tsv`
  (`9b9ada2358d0038bf2b8caac1be3143f46127d2ac73a3fae81eb12545bf17c31`).

Binary identities:

- current baseline:
  `dda826459cfa9cb017b751749d2b780419b18cc1a2ff9ff309492ea8b4df61da`;
- current bounded candidate:
  `eefeb5e10c605d3c89791717f9e4500ca764c4c653abec04463eaf9124958e96`;
- balanced baseline:
  `06ef3278e9055f3c53c94cf964f4a7bf785453b696e0df262dec9161b45c6ab8`;
- balanced bounded candidate:
  `c053614625760b714e21ea7f1eabeb18ad44544191d083716d2b6a6fd4c774dd`.

## Decision

**Rejected.** The balanced encode regression exceeds the 1% gate by an order
of magnitude and outweighs a decode improvement that still does not surpass
the practical-compression frontier. The candidate is dominated, so the slow
four-case confirmation and frontier update are intentionally skipped. The
working codec retains the original distance formulation.
