# EXP-0084 — Specialized Rice batching

Status: **REJECTED**

## Classification

**Speed exploitation** — specialize the exact four-symbol packer from
EXP-0083 for the matched q90 branch's observed Rice-0 and Rice-4 tiles.

## Hypothesis

EXP-0083 improved q90 one-thread encoding 3.16%, but its group packer retains
a runtime Rice parameter in every quotient, mask, length, and shift. The
matched q90 stream uses zero-run, Rice-0, and Rice-4 tiles. Dispatching once
per tile to const-generic Rice-0/Rice-4 loops should let LLVM constant-fold
the per-group arithmetic and raise the portable improvement above 5% while
remaining byte-identical.

## Modification

Starting from the exact EXP-0083 source:

1. dispatch Rice parameters 0 and 4 outside the causal pixel loop;
2. compile separate const-generic four-symbol predictor/writer loops;
3. retain the original EXP-0078 per-symbol path for every other parameter;
4. retain exact long-group and row-remainder fallback; and
5. change no parameter selection, syntax, decoder, allocation, quality, or
   target CPU.

The candidate remains safe, portable Rust. A writer test must compare each
specialized group with scalar output over alignments and fallback values.

## Fast test

Run the focused six-trial q90/q100 A/B against EXP-0078, not EXP-0083.
Require exact bytes and metrics. Report entropy modes to verify the matched
q90 path exercises only the declared specialized Rice parameters. If the
one-thread q90 gate passes, profile and confirm on the complete supplement.

## Gate

- at least 5% matched q90 one-thread encode improvement over EXP-0078;
- byte- and metric-identical q90/q100 streams;
- q90 decode no worse than 5%;
- no q90 four-thread encode regression beyond 5%;
- strict Clippy, formatting, and specialized writer equivalence pass; and
- no OpenAPV confirmation unless the focused gate passes.

## Result

The specialized writer equivalence test passed for Rice-0/Rice-4, alignments
0--7, short groups, word crossings, and long-code fallback. The focused
candidate remained byte- and metric-identical:

| Quality | Threads | EXP-0078 encode | Specialized encode | Change |
|---:|---:|---:|---:|---:|
| 90 | 1 | 66.858 MP/s | 69.353 MP/s | +3.73% |
| 90 | 4 | 183.584 MP/s | 194.428 MP/s | +5.91% |
| 100 | 1 | 63.450 MP/s | 64.327 MP/s | +1.38% |
| 100 | 4 | 176.930 MP/s | 179.624 MP/s | +1.52% |

The q100 result is intentionally smaller than EXP-0083 because parameters
other than 0 and 4 use the original scalar path. q90 retained 18,882,860
bytes, 52.001930 dB Y-PSNR, 0.99373056 SSIM, and maximum error 4; q100
retained 32,246,235 bytes and exact reconstruction. One-thread q90 decode
changed from 64.154 to 62.395 MP/s (-2.74%); four-thread decode changed
-0.37%.

Const specialization improves both q90 encode cells, but the primary
one-thread gain remains below the 5% advancement gate. Strict Clippy,
formatting, and the specialized equivalence test passed. The full corpus,
profile, and OpenAPV matrix were skipped according to the fast gate.
Production source was restored exactly.

Artifacts:

- focused matrix:
  `artifacts/exp0084-specialized-focused.tsv`
  (`b1bb25ff4cc1fd28e3e86a912360cafe96ae2d4bf06698869f2091b9cd8d4ab8`);
- release binary:
  `artifacts/frontier/fastvid-speed-exp0084-specialized`
  (`82ad1ab34c643790cbbf2a36ce30084c1966fbd9a909a9e48a2cd682d9d5c3d9`);
- exact source patch:
  `artifacts/frontier/exp0084-specialized-rice4.patch`
  (`38631446b783cd0c96285509f488ae8e1fb56eecacf770b73ce10976f43f6c73`).

## Decision

Reject as a standalone frontier replacement and retain EXP-0078. Tile-level
constant dispatch is useful but does not make four-symbol packing large
enough to pass the primary gate.

Do not immediately stack group-size variants. The combined EXP-0083/0084
evidence says batching is directionally sound but bounded; another profile or
an algorithm that emits substantially more than four symbols per state
update is required. Any future combination with the host-native EXP-0082
build must first pass as a portable source improvement.

## References

- [EXP-0079](EXP-0079-unified-speed-profile.md)
- [EXP-0080](EXP-0080-inlined-rice-writer.md)
- [EXP-0083](EXP-0083-four-symbol-rice-batching.md)
