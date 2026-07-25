# EXP-0095 — Block-pack plus specialized Rice-4

Status: **ACCEPTED**

## Classification

**Tech-tree synthesis** — combine two orthogonal, byte-exact speed
optimizations that have not previously shared a candidate.

## Hypothesis

EXP-0084's Rice-0/Rice-4 four-symbol specialization improved the former
frontier's matched q90 one-thread encode by 3.73% but missed its original 5%
gate. EXP-0088 later accelerated block-packed tiles without changing legacy
Rice tiles. Applying the specialization only to the remaining Rice-0/Rice-4
tiles should retain at least a 2% improvement over EXP-0088 with identical
streams and quality.

## Modification

1. Dispatch Rice parameters 0 and 4 once per selected tile.
2. Reuse EXP-0084's exact const-generic four-symbol packing kernel.
3. Retain EXP-0088's block packer and scalar Rice path for all other
   parameters.
4. Change no selector, entropy mode, syntax, decoder, quality mapping, or
   tile geometry.

## Gate

- specialized writer equivalence across alignments, short groups, and
  fallback values;
- byte- and metric-identical focused q90/q100 streams;
- at least 2% matched q90 one-thread encode improvement over EXP-0088;
- no focused encode cell regresses more than 3%;
- decode no worse than 5%;
- strict Clippy, formatting, and relevant release tests pass; and
- no slow tier unless the focused gate passes.

## Result

The specialized writer equivalence test passed for Rice-0/Rice-4, bit
alignments 0--7, short groups, word crossings, and long-code fallback. Strict
release Clippy and formatting passed.

The six-trial focused matrix was byte- and metric-identical:

| Depth | Quality | Threads | Baseline encode | Candidate encode | Delta | Decode delta |
|---:|---:|---:|---:|---:|---:|---:|
| 10 | 90 | 1 | 71.317 MP/s | 75.253 MP/s | +5.520% | -0.635% |
| 10 | 90 | 4 | 183.257 MP/s | 198.385 MP/s | +8.255% | +0.437% |
| 16 | 90 | 1 | 67.920 MP/s | 68.472 MP/s | +0.813% | +0.063% |
| 16 | 90 | 4 | 176.889 MP/s | 173.833 MP/s | -1.728% | -4.488% |
| 10 | 100 | 1 | 65.154 MP/s | 64.842 MP/s | -0.478% | +1.267% |
| 10 | 100 | 4 | 184.186 MP/s | 182.107 MP/s | -1.128% | -0.666% |
| 16 | 100 | 1 | 62.654 MP/s | 62.077 MP/s | -0.920% | -0.482% |
| 16 | 100 | 4 | 164.950 MP/s | 172.700 MP/s | +4.698% | +3.841% |

The q90 encode geomean improved 3.140% at one thread and 3.143% at
four threads. No focused encode cell regressed by 3%, and decode stayed
within 5%.

Six-trial confirmation on the complete native supplement, including
1920x1080 HDR-gradient and precision-UI assets, measured:

| Quality | Threads | Encode delta | Decode delta |
|---:|---:|---:|---:|
| 90 | 1 | +4.183% | +0.750% |
| 90 | 4 | +4.089% | +1.756% |
| 100 | 1 | -0.453% | -0.728% |
| 100 | 4 | +0.401% | +3.573% |

All complete-corpus streams and metrics were identical. The largest complete
encode regression was -3.165% on 10-bit q100 one-thread; q90, the declared
speed operating point, improved on seven of eight cells and regressed only
0.271% on 12-bit UI at four threads.

The first six-trial random-access matrix retained identical encoded bytes,
dependency frames, and decoded work. Its noisy aggregate medians motivated a
ten-trial matched q90/10-bit rerun. Every target-frame latency change then
stayed within 5% (range -1.110% to +4.504%); geometric aggregate latency was
+2.208%, and useful/work throughput was -2.160%.

The full candidate suite passed 52/57 tests. A clean export of baseline
source passed 51/56 and reproduced the exact same five pre-existing
selector-policy failures. The candidate's additional specialized-writer test
passed, and it introduced no new failure.

Artifacts:

- complete native confirmation:
  `artifacts/exp0095-block-rice4-complete.tsv`
  (`5984e6370cc7a72b58a4658230c1cbd438e1919d0c1a29b0bf9a7966b3af3551`);
- focused matrix:
  `artifacts/exp0095-block-rice4-focused.tsv`
  (`f18ffc112494fd0cfc51d33236fa0bc97a5f3652cc954b5e7a7e30c2747e879b`);
- complete access matrix:
  `artifacts/exp0095-block-rice4-access.tsv`
  (`54df8d40fab3f98d5c2c7c199e334facd6f9299a662096247e41f90ba059da67`);
- ten-trial matched access rerun:
  `artifacts/exp0095-block-rice4-access-q90-10bit.tsv`
  (`9a0458387874f6b42cdd816fd22c52ff8de3da24a5c2e9e6f8aaedd5ff616641`);
- candidate binary:
  `artifacts/frontier/fastvid-speed-exp0095-block-rice4`
  (`637ee0535510f38dd9dc99f02fc5acbd75f7d927f5b2a3517d2b8f4b167c1407`);
- candidate codec source:
  `src/codec16.rs`
  (`da2eb1cf222f5d29cc9526194437ec75c9da58d215aaca11555600376fb31a87`).

## Decision

Accept the synthesis. It exceeds the current 2% advancement gate on the
matched q90 path, improves complete q90 encode throughput, preserves every
byte and quality metric, and remains within decode/access tolerances.

This result reverses neither EXP-0084 nor its original 5% decision. The new
evidence is specifically that specialization composes favorably with the
later block-pack frontier, and its current marginal value is large enough to
promote.

## References

- [EXP-0083](EXP-0083-four-symbol-rice-batching.md)
- [EXP-0084](EXP-0084-specialized-rice-batching.md)
- [EXP-0088](EXP-0088-portable-block-pack-kernel.md)
- [EXP-0090](EXP-0090-post-pack-speed-profile.md)
