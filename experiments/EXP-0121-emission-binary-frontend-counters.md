# EXP-0121 — Emission binary frontend counters

Status: **ACCEPTED**

## Classification

**Measurement / methodology exploration** — investigate the unchanged-decoder
regressions in rejected EXP-0114 and EXP-0120 before testing another Rice
writer.

## Hypothesis

The direct-writer candidate changes no decoder source or stream bytes, yet its
whole-binary decode timing regresses similarly to EXP-0114. If code layout or
instruction working-set effects are responsible, the candidate's complete
encode/decode process should show a materially higher L1 instruction-cache
miss count or frontend-stalled-cycle count than the exact EXP-0117 binary.
If those counters do not move coherently, combined-process timing or host
frequency remains the stronger explanation and separate-process decode
measurement is required.

## Modification

No source modification. Use the preserved exact EXP-0117 and rejected
EXP-0120 binaries on the same 24-frame 1280x720 10-bit q90 motion sequence.
Collect repeated cycles, reference cycles, instructions, L1 instruction-cache
loads/misses, and frontend-stalled cycles for the complete benchmark process.

## Test

- verify both binary hashes and exact benchmark stream bytes;
- collect at least three repetitions per binary;
- reject any unsupported or zero-valued counter as evidence;
- compare normalized frontend stalls and I-cache misses, not raw wall time
  alone;
- use the result to decide whether another writer implementation is justified
  or a decode-only harness is the next prerequisite.

## Result

Both hashes match their EXP-0120 record, and both binaries retain the same
18,502,889-byte q90 stream. Five `perf stat` repetitions per binary report
the following complete-process means:

| Counter | EXP-0117 reference | EXP-0120 candidate | Candidate/reference |
|---|---:|---:|---:|
| cycles | 4,377,352,492 | 4,129,065,132 | 0.9433x |
| reference cycles | 2,653,489,638 | 2,495,761,774 | 0.9406x |
| instructions | 17,279,627,575 | 15,905,303,567 | 0.9205x |
| L1-I loads | 75,241,554 | 90,381,605 | 1.2012x |
| L1-I load misses | 1,591,699 | 1,438,953 | 0.9040x |
| L1-I misses / million instructions | 92.114 | 90.470 | 0.9822x |
| cycles / reference cycle | 1.649659 | 1.654431 | 1.0029x |

The candidate has 9.60% fewer raw L1-I misses and 1.78% fewer misses per
instruction, contradicting the proposed instruction-cache-miss explanation.
Its cycles/reference-cycle ratio differs by only 0.29%, providing no evidence
of a material frequency shift. It also executes 7.95% fewer instructions and
5.67% fewer cycles for the complete process, consistent with its faster
encoder.

`stalled-cycles-frontend` reports zero for both binaries and is rejected as an
unsupported or unusable host counter. L1-I loads rise while misses fall; the
event's exact virtualized-host semantics are insufficient to infer a decoder
frontend regression from loads alone.

The sequential diagnostic's median decoder ratio is approximately 0.952x,
less severe than the balanced EXP-0120 result but still near the acceptance
boundary. Aggregate complete-process counters cannot attribute that phase
movement.

Artifacts:

- `artifacts/exp0121-reference-frontend-stat.tsv`
  (`b4cd34c618d3a1770b88047a2c417ee8a38158be88cb8e87d083775fd1b2046b`);
- `artifacts/exp0121-candidate-frontend-stat.tsv`
  (`031aabd0292bfc565ffa3d40659e133a66422ebfb6ba38d30343abf1642bff60`).

## Decision

Accept the counter result as negative evidence: neither L1-I misses nor
frequency explains the unchanged-decoder timing regression at the complete
process level. Do not use the zero frontend-stall count.

Before revisiting Rice emission, add a decode-only benchmark entry point that
loads and validates a fixed encoded sequence before the timed region, decodes
it repeatedly in a fresh process, and alternates separately preserved
binaries. Encode-only and decode-only phase measurements should supplement,
not replace, the existing whole-codec gate. This is now a methodology
prerequisite because aggregate counters and combined timing cannot distinguish
layout effects inside the decoder from cross-phase or harness interaction.

## References

- [Research 0012](../research/0012-simd-cache-profiling.md)
- [EXP-0114](EXP-0114-parallel-rice-grouped-emission.md)
- [EXP-0118](EXP-0118-post-paired-rice-profile.md)
- [EXP-0120](EXP-0120-direct-rice-lane-emission.md)
