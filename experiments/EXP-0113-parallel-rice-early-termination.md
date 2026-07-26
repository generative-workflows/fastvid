# EXP-0113 — Parallel Rice early termination

Status: **ACCEPTED**

## Classification

**Version-5 speed exploitation** — optimize the 68.34% search hotspot found
by EXP-0112 without changing format decisions.

## Hypothesis

Stopping exact four-lane Rice parameter search when every lane's quotient sum
is zero should preserve the selected parameter and stream byte-for-byte.
Bounded q90 residuals should reach this condition well before parameter 16,
improving geometric native version-5 encode throughput by at least 1.35x
without changing decode throughput beyond 5%.

## Modification

Replace the unconditional 0–16 `min_by_key` scan with an explicit ascending
loop. Retain exact per-lane byte-rounded costs and first-minimum tie behavior.
Stop after evaluating the first parameter whose aggregate quotient sum is
zero: every higher parameter has zero quotient and strictly greater
remainder cost, so it cannot win.

## Gate

- exhaustive single-value and representative multi-lane tests match the
  complete 17-parameter scan;
- the EXP-0110 control hash and every native q90 encoded byte remain exact;
- candidate-only feedback versus fixed EXP-0110 rows reaches at least 1.35x
  geometric encode throughput and at least 0.95x decode throughput;
- full release tests, strict Clippy, formatting, and diff checks pass.

## Result

The exact early-termination selector matches a complete byte-rounded
four-lane scan for every individual folded value from 0 through 131,070 and
for deterministic multi-lane sequences of 2, 3, 4, 5, 127, 128, 4,095, and
4,096 symbols. The version-5 HDR control retains EXP-0110's exact SHA-256
`9a3cf708ecdc73f9f8c15a545b41f761ad1ed844c2b8cb4db42118ce587fce37`.

Three candidate-only q90 trials were compared with the fixed EXP-0110 rows:

| Sample | Encode | Encode ratio | Decode ratio |
|---|---:|---:|---:|
| HDR gradient 10 | 19.184 MP/s | 1.611x | 0.950x |
| Precision motion 10 | 19.880 MP/s | 1.670x | 0.973x |
| Precision UI 12 | 20.593 MP/s | 1.704x | 0.979x |
| Precision motion 16 | 25.393 MP/s | 1.953x | 0.974x |
| **Geometric** | — | **1.7299x** | **0.9689x** |

Every encoded byte and bitrate is unchanged. The encode result clears the
1.35x gate by a wide margin. Decoder code is unchanged; its fixed-reference
timing ratio remains within the 5% tolerance geometrically.

A 40-repeat post-change cycle profile captured 17K samples with none lost.
Inlining combines the remaining selector and predictor work in the version-5
tile closure (76.67%); Rice emission is now 16.13%, fixed block 2.41%, frame
validation 1.30%, AVX-512 `memmove` 1.04%, and allocator self-time 0.51%.

Artifacts:

- `artifacts/exp0113-rice-early-termination.tsv`
  (`6bdd3af7e7ae523db7ec449f4f9a0a891238ad31bd3fb51a9fd76be6c823743c`);
- `artifacts/exp0113-v5-encode-perf.data`
  (`f8df44013a77e6a14b80b1793492ac0186770d4437fd14552f63873acbeb58de`);
- `artifacts/exp0113-v5-encode-perf-report.txt`
  (`e0e6611777c56e230ee6480e0a05fb448fa63d75d8df33e5b87bb974f4d30cd6`).

## Decision

Accept exact Rice early termination. It is byte-identical, broadly effective
across 10/12/16-bit content, and raises the leading parallel candidate from
roughly 0.174x to 0.301x version-2 encode throughput without moving its rate
or quality point.

Keep version 5 non-promoted: approximately 19–25 MP/s is still far below the
CPU speed tier and OpenAPV fastest. The next exploration should separate the
now-inlined tile closure with a microbenchmark or debug-symbol profile and
compare an exact one-pass quotient histogram against a mean-guided narrow
Rice search. Rice emission is independently large enough (16.13%) to justify
a later specialized four-lane writer after selection cost is understood.

## References

- [Research 0027](../research/0027-streaming-rice-parameter-selection.md)
- [EXP-0110](EXP-0110-full-tile-bounded-shards.md)
- [EXP-0112](EXP-0112-version5-encode-profile.md)
