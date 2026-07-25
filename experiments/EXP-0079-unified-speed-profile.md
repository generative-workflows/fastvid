# EXP-0079 — Unified high-bit speed profile

Status: **ACCEPTED**

## Classification

**Speed exploitation diagnosis** — profile the promoted EXP-0078 speed
frontier at the matched q90 operating point before selecting its next kernel
optimization.

## Hypothesis

EXP-0077 removed most buffered entropy-finalization work from dense Rice
tiles. The remaining one-thread encode deficit to OpenAPV `fastest` should
therefore be concentrated in one or more of:

- causal clamp-gradient prediction and reconstructed-row updates;
- quantization and Rice bit emission;
- sparse-tile exact fallback;
- per-tile allocation and result assembly; or
- work outside the measured encode interval.

Cycle sampling and hardware counters can distinguish these explanations and
identify a kernel large enough to plausibly close a material part of the
remaining 16.32% deficit.

## Measurement

Use the preserved EXP-0078 speed binary on the checksummed
1280x720x24 native-10-bit procedural sequence at q90, GOP 1, 256x128 tiles,
and one thread:

1. verify the binary and source-input hashes;
2. warm the command once;
3. collect five `perf stat` trials for cycles, instructions, branches,
   branch misses, and cache references;
4. collect a 999 Hz userspace call-graph profile;
5. report exclusive and inclusive encode hotspots separately from decode and
   metric work; and
6. retain raw counters, profile data, report, and hashes.

The CLI command includes encode, decode, and metrics, so symbol shares are
whole-command shares. Do not infer cache misses from an unavailable or
invalid host event.

## Gate

Advance an implementation direction only when:

- at least 15% of visible encode samples belong to a concrete optimizable
  kernel, or a measured instruction/cycle result supports a bounded fusion;
- the proposed change preserves reconstruction and the declared q90 quality
  boundary; and
- the direction has a plausible fast-feedback test that does not require the
  full OpenAPV matrix.

This experiment changes no production source.

## Result

Five hardware-counter trials were stable:

| Counter | Median | Relative spread |
|---|---:|---:|
| cycles | 2,998,969,424 | 0.66% |
| instructions | 13,055,405,631 | 0.01% |
| branches | 2,246,300,557 | 0.01% |
| branch misses | 18,304,001 | 0.09% |
| cache references | 80,278,845 | 1.08% |

The complete CLI command took 0.8623 seconds. These counters include encode,
decode, validation, and metrics and therefore establish a stable boundary
rather than encode-only costs.

The 999 Hz userspace profile captured 834 cycle samples with none lost.
Largest exclusive symbols were:

| Symbol | Whole-command samples |
|---|---:|
| `codec16::decode_tile_payload` | 19.59% |
| unified high-bit encode closure | 18.17% |
| `codec16::reconstruct` | 17.73% |
| `codec16::BitWriter::put_rice` | 17.25% |
| `metrics::compare_plane16` | 5.99% |
| `model::Frame16::validate` | 5.65% |
| `metrics::ssim_plane16_sampled` | 5.57% |
| `codec16::finish_entropy` | 3.68% |

The visible encode hot set is approximately the 39.10% sum of the encode
closure, direct Rice writer, and sparse fallback. `put_rice` is therefore
about 44% of that set. EXP-0077 achieved its intended redistribution:
`finish_entropy` fell from 25.58% in EXP-0076 to 3.68%, while direct bit
emission became the largest concrete encode kernel.

Disassembly explains why this safe scalar writer is unusually expensive.
`put_rice` remains a non-inlined call for every coded residual, with a
six-register prologue/epilogue. Its common path also performs the
`Vec::push` capacity check and byte-at-a-time flush loop on each call. The
annotation assigned substantial local samples to writer state loads/stores,
capacity/length handling, and function return overhead; this supports a
bounded inline/bulk-flush experiment before changing prediction or syntax.

Artifacts:

- hardware counters:
  `artifacts/exp0079-perf-stat.txt`
  (`2ea570fb8e6b4075ee0f238ca7fc9840fbbd4725a89f7c86a2655eff1de58f89`);
- sampled profile:
  `artifacts/exp0079-perf.data`
  (`36e4e3cb3228e83b5635600c7cb15ed010730df438784c8d437d09c69e1812d5`);
- exclusive report:
  `artifacts/exp0079-perf-report.txt`
  (`462e22a7a2fb5d32c2a7a05f83a7c7d278239718b6458bbc800c9785f7fc2ea4`);
- inclusive report:
  `artifacts/exp0079-perf-report-children.txt`
  (`be60dc666ac9f17eac7e56378ab23623360710461d981e14a49553818553cd42`);
- EXP-0078 binary:
  `bf1002e7e790bb5607180ff2874edd57957536c83cce620982f0a6999614ccb3`;
- source input:
  `ff61ed1af3c39e4b12e8a98a8edb94b2d76e2dfcc2f318a62e111b7080b5fbad`.

## Decision

Accept as actionable diagnostic evidence. Advance an isolated speed-branch
experiment that first forces the Rice writer's common path inline, then
screens a bulk byte flush if inlining alone is insufficient. It must preserve
encoded bytes exactly and use a short focused A/B before any matched OpenAPV
confirmation.

SIMD clamp-gradient remains the next independent direction: prediction and
reconstructed-row work is still 18.17% of the whole command. It should not be
mixed into the writer experiment because causal left-neighbor dependence and
bitstream changes require separate attribution.

## References

- [Research 0012](../research/0012-simd-cache-profiling.md)
- [Research 0019](../research/0019-modern-integer-entropy-kernels.md)
- [EXP-0076](EXP-0076-fixed-high-bit-perf-profile.md)
- [EXP-0077](EXP-0077-high-bit-prefix-rice-streaming.md)
- [EXP-0078](EXP-0078-unified-speed-frontier.md)
