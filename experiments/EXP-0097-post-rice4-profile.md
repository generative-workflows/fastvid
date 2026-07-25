# EXP-0097 — Post-Rice-4 speed profile

Status: **ACCEPTED**

## Classification

**Profiling exploration** — locate the remaining matched q90 encode cost
after EXP-0095/0096 reduced the OpenAPV deficit to 6.07%.

## Hypothesis

Four-symbol Rice batching should materially reduce the previous 26.51%
per-symbol writer share. The fused causal predictor/reconstruction loop
should now dominate, while fixed-block and batched Rice emission together
remain below 20% of encode-only samples.

## Test

1. Use source `91a755e` and the encode-only driver from EXP-0090.
2. Preconstruct all 24 matched q90 frames and encode four repetitions at one
   thread, 256x128 tiles.
3. Record hardware counters and a call-graph cycle profile.
4. Retain raw data and an exact symbol report.
5. Select the next experiment only from a current cost center large enough
   to plausibly close the remaining approximately 6.5% relative gap.

## Result

Four repetitions reproduced exactly 73,584,828 bytes, or 18,396,207 bytes
per 24-frame sequence. Six hardware-counter repetitions averaged:

- 5.165 billion cycles;
- 17.518 billion instructions, approximately 3.39 IPC;
- 104.662 million cache references and 5.407 million misses (5.17%); and
- 2.815 billion branches.

The virtual PMU again reported zero branch misses, so that counter remains
unusable. No samples were lost from the call-graph profile.

The 6,318-sample encode-only self profile was:

| Symbol | Self cycles |
|---|---:|
| fused `encode_internal` tile closure | 41.06% |
| scalar `BitWriter::put_rice` | 28.80% |
| `finish_entropy` | 7.37% |
| frame validation | 4.91% |
| fixed-block writer | 2.45% |

Contrary to the hypothesis, the scalar writer share did not fall below 20%;
it rose from EXP-0090's 26.51% sample share. Direct inspection of the first
matched frame found 45 tiles divided exactly among 15 Rice-0, 15 zero-run,
and 15 block-pack modes. The specialization is therefore exercised, but its
current all-or-nothing group fallback still invokes the scalar writer
frequently.

The four-symbol kernel discards an already packable prefix whenever the next
code would push the group over 64 bits, then writes all four values
individually. Preserving that prefix and falling back only from the first
overflowing symbol is the smallest current-profile-directed follow-up.

Artifacts:

- hardware counters:
  `artifacts/exp0097-perf-stat.tsv`
  (`3f03b8e2d5fdf4fdfa9aad627d0ff5c6abfcf2861a4cbbf07aa08cd33f16e78f`);
- raw cycle profile:
  `artifacts/exp0097-encode-perf.data`
  (`ca078da4762b28777f4255099f4df21f213f6e17fd6e0b87e827aca2f4e30108`);
- symbol report:
  `artifacts/exp0097-encode-perf-report.txt`
  (`e6d57dd90aa2c4665b1276b3b5f280d4d51752d5f63736c479e1eee5c74cc193`);
- encode-only driver binary:
  `target/release/encode16_profile`
  (`1cebfdc891336a35680cdf488d560ddf161993bae380753d93752eab6cccee40`).

## Decision

Accept the profile and reject its stated expectation that batching had made
Rice emission secondary. The two current targets large enough to close the
remaining gap remain the fused causal loop and Rice writing.

Exploit the newly exposed partial-group fallback first: it preserves the
accepted kernel and exact syntax while potentially reducing calls within a
28.80% cost center. If it fails, return to a predictor-loop layout change
rather than more function-inlining or quantizer variants already rejected.

## References

- [Research 0012](../research/0012-simd-cache-profiling.md)
- [EXP-0090](EXP-0090-post-pack-speed-profile.md)
- [EXP-0095](EXP-0095-block-pack-rice4-combination.md)
- [EXP-0096](EXP-0096-rice4-speed-promotion.md)
