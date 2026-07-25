# EXP-0090 — Post-pack speed profile

Status: **ACCEPTED**

## Classification

**Profiling exploration** — locate the remaining practical-q90 encode cost
after EXP-0088 before selecting an architecture-specific optimization.

## Hypothesis

The portable fixed-block writer is no longer a dominant encode cost.
Fixed-gradient prediction/reconstruction and Rice writing for the two
non-packed planes should account for most remaining samples. A fresh profile
of source `29dcc43` should identify a kernel with enough inclusive cost to
close the remaining 12.18% OpenAPV encode deficit.

## Test

1. Record kernel policy and available hardware counters.
2. Run `perf stat` on the pinned matched q90 one-thread command and report
   cycles, instructions, IPC, branches/misses, and cache references/misses
   when permitted.
3. Run a call-graph sample of the same command with the promoted release
   binary and retain raw/report artifacts.
4. Add a separate all-intra encode-only profiling driver that preconstructs
   frames and invokes the public encoder without decode, metrics, or timed
   input parsing; use it to separate encode costs cleanly.
5. Select the next experiment only from a function or fused loop with at
   least 15% relevant inclusive cost; do not optimize fixed packing merely
   because it now has an available SIMD shape.

## Result

`perf_event_paranoid` is now 1 and user-space hardware counters are
available. Six whole-command repetitions measured:

- 2.913 billion cycles;
- 11.986 billion instructions, or approximately 4.12 IPC;
- 80.623 million cache references and 3.403 million misses (4.22%); and
- 2.102 billion branches.

The virtual PMU reported exactly zero branch misses even for the control
command, so that counter is not credible and is excluded from decisions.
Kernel symbols remain restricted, but no samples were lost and user-space
symbols resolved.

The whole benchmark includes decode and metrics. Its leading self costs were
19.66% encode closure, 18.60% tile decode, 14.62% reconstruction, 11.59%
Rice writing, 5.39% comparison, 5.15% frame validation, and 5.02% sampled
SSIM. Fixed-block writing was only 0.84%.

The encode-only driver preconstructed all 24 frames, reproduced exactly
18,396,207 encoded bytes per repetition, and ran four repetitions to
amortize setup. Its 3,014-sample profile was:

| Symbol | Self cycles |
|---|---:|
| fused `encode_internal` tile closure | 42.82% |
| `BitWriter::put_rice` | 26.51% |
| `finish_entropy` | 7.80% |
| frame validation | 4.51% |
| fixed-block writer | 2.44% |

This rejects fixed-block SIMD as the next priority. The annotation of
`put_rice` shows a six-register save/restore sequence and a 0x250-byte
function containing both its common fitting-code path and rare overflow
loops. Register saves/restores alone received a large fraction of its local
samples. EXP-0080's forced inline duplicated the entire function and
regressed; the new evidence instead supports splitting a tiny inline common
path from a cold non-inlined overflow helper.

Artifacts:

- hardware-counter matrix:
  `artifacts/exp0090-perf-stat.tsv`
  (`ca1bc4ebad330482c45725361399179b12270b43db37595a6bddfe857f262862`);
- whole-command profile:
  `artifacts/exp0090-perf.data`
  (`b0efc3174591c9d337ad0364249a3d21aa8d221bde7c260553d947af5e26e576`);
- encode-only profile:
  `artifacts/exp0090-encode-perf.data`
  (`d481d5476183ce3b9964b09ed682987774632cdb6a297ab3b4e6183b703986dc`);
- encode-only driver:
  `src/bin/encode16_profile.rs`
  (`d87ce4555ca2d8f13b8df90c2b35a348bbddaa58f2d2c712fdca612d04366439`).

## Decision

Accept the profile. It identifies two relevant targets above the declared
15% threshold: the dependency-heavy fused prediction loop and Rice writing.
Exploit the clearer Rice opportunity first by splitting its common path from
the slow overflow path. This is distinct from EXP-0080's rejected whole-body
inlining and from the rejected multi-symbol batching experiments.

Only if that fast split fails should the next branch tackle the fused
predictor loop through state/layout changes; its causal left/above dependency
makes direct lane-wise SIMD unlikely.

## References

- [Research 0012](../research/0012-simd-cache-profiling.md)
- [Research 0014](../research/0014-sampling-and-high-bit-quantization.md)
- [EXP-0079](EXP-0079-unified-speed-profile.md)
- [EXP-0088](EXP-0088-portable-block-pack-kernel.md)
- [EXP-0089](EXP-0089-portable-kernel-speed-promotion.md)
