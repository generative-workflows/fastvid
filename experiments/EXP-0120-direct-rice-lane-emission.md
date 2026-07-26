# EXP-0120 — Direct Rice lane emission

Status: **REJECTED**

## Classification

**Version-5 speed exploration/exploitation** — target the 23.60% Rice
emission hotspot and reduce per-shard memory traffic in a
parallel-hardware-friendly way.

## Hypothesis

The exact selector already computes every winning lane's bit count, but
production discards those lengths, grows four independent byte vectors, then
copies all four into a final body. Returning the winning byte lengths should
permit one exact body allocation and four disjoint safe slice writers.

Writing directly to final lane spans should preserve the stream byte-for-byte
and improve geometric encode throughput by at least 1.05x over EXP-0117,
while retaining at least 0.95x decode throughput. It should also remove four
lane allocations and all serialized lane-to-body payload copies per Rice
shard, which better matches an eventual count/scan/disjoint-write GPU pass.

## Modification

- retain the winning parameter and four byte-rounded lane sizes from the
  exact paired selector;
- allocate the final Rice body once, write the three lane lengths into its
  prefix, and partition the payload into disjoint exact-size slices;
- emit each lane with a safe bounded bit writer over its assigned slice;
- assert that every writer consumes its exact selected length.

Do not change parameter selection, lane assignment, code ordering, padding,
shard selection, prediction, reconstruction, or syntax.

## Test

- extend direct Rice tests across short, long, extreme, and lane-tail inputs;
- retain the exhaustive full-scan selector oracle and accepted control hash;
- run five balanced alternating trials against the exact EXP-0117 binary;
- require at least 1.05x geometric encode and 0.95x geometric decode
  throughput;
- require identical bytes, bitrate, PSNR, SSIM, and maximum error;
- run the full release suite, normal and profiling-feature strict Clippy,
  formatting, and diff checks.

## Result

The direct writer matches the vector reference for 1, 2, 3, 4, 5, 127, 128,
4,095, and 4,096-symbol inputs containing zero, maximum folded residuals, and
lane tails. The exhaustive selector oracle remains exact, and every native
q90 output byte and bitrate is unchanged.

Five balanced whole-binary trials measured:

| Sample | Candidate encode | Encode ratio | Decode ratio |
|---|---:|---:|---:|
| HDR gradient 10 | 30.648 MP/s | 1.076x | 0.957x |
| Precision motion 10 | 32.555 MP/s | 1.084x | 0.936x |
| Precision UI 12 | 35.736 MP/s | 1.092x | 0.950x |
| Precision motion 16 | 50.046 MP/s | 1.153x | 0.894x |
| **Geometric** | — | **1.1007x** | **0.9337x** |

The direct layout clears the encode gate substantially but fails the complete
binary decode gate. Decoder source and stream bytes are unchanged, so the
decode movement is an implementation-layout, instruction-cache, host
frequency, or combined-process interaction rather than a format cost.
Nevertheless, it reproduces EXP-0114's directionally identical whole-binary
regression and is authoritative under the current methodology.

The fixed EXP-0117 binary has SHA-256
`df4818b6b296103862277c50e1245703db7c9e2ee24d4e133fe4541d8659dcc6`;
the candidate binary has SHA-256
`220d79c231106e00226a8e86b0ee063742177c02ef0c0148a7593cf9fd597196`.
The balanced artifact is
`artifacts/exp0120-direct-rice-confirm.tsv`
(`c394614c9c871ae36f6703644995b697e32be86741befbff8816fd5ec7b9f14d`).
The direct reference oracle, exhaustive selector oracle, and strict normal
Clippy pass.

## Decision

Reject and revert this CPU implementation because 10.07% encode gain does not
justify a measured 6.63% decode loss.

Retain the architectural result: exact selector lengths are sufficient for
one allocation and four disjoint final writes, eliminating serialized lane
copies. That is directly useful to the CUDA branch even though this scalar
binary does not clear the balanced CPU gate.

Before a third CPU emission attempt, add separate-process encode-only and
decode-only confirmation with instruction-cache or frontend counters. Two
independent implementations have now produced byte-identical encode gains
alongside decoder-source-invariant whole-binary regressions; more writer
variants without resolving that measurement mechanism would be low-value.

## References

- [Research 0039](../research/0039-parallel-rice-bitstream-hardware.md)
- [EXP-0114](EXP-0114-parallel-rice-grouped-emission.md)
- [EXP-0117](EXP-0117-paired-rice-parameter-pass.md)
- [EXP-0118](EXP-0118-post-paired-rice-profile.md)
