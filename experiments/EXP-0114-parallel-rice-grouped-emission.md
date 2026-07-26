# EXP-0114 — Parallel Rice grouped emission

Status: **REJECTED**

## Classification

**Version-5 speed exploitation** — reuse the proven four-code writer in the
16.13% post-EXP-0113 Rice emission hotspot.

## Hypothesis

Writing four consecutive symbols from each version-5 Rice lane per bit-writer
update for common parameters 0 and 4 should preserve every byte and improve
geometric native q90 encode throughput by at least 5%. Decoder throughput
must remain within 5%.

## Modification

For each strided Rice lane, gather four lane-consecutive folded values and
call the existing exact `put_rice4_specialized` kernel when the selected
parameter is 0 or 4. Emit the lane tail and all other parameters through the
scalar writer. Do not change lane assignment, parameter selection, padding,
or syntax.

## Gate

- direct kernel and complete control-stream byte identity pass;
- native q90 bytes, bitrate, and quality remain exact;
- candidate-only feedback versus fixed EXP-0113 rows reaches at least 1.05x
  geometric encode throughput and at least 0.95x decode throughput;
- full validation passes.

## Result

The grouped writer is byte-identical in direct lane-tail tests and preserves
the EXP-0110/0113 control hash
`9a3cf708ecdc73f9f8c15a545b41f761ad1ed844c2b8cb4db42118ce587fce37`.
Native bytes, bitrate, and quality do not move.

The three-trial candidate-only fast screen measured 1.0539x geometric encode
and 0.9241x fixed-reference decode. Because decoder source was unchanged, a
slow confirmation built the exact `9ed1337` predecessor in a separate source
tree and alternated predecessor/candidate order over five trials:

| Sample | Encode ratio | Decode ratio |
|---|---:|---:|
| HDR gradient 10 | 1.070x | 0.954x |
| Precision motion 10 | 1.071x | 0.926x |
| Precision UI 12 | 1.039x | 0.966x |
| Precision motion 16 | 1.086x | 0.908x |
| **Geometric** | **1.0665x** | **0.9381x** |

The encode gate passes but the full-binary decode gate fails. Keeping the
generic lane kernel out of line did not recover it: a three-trial screen
measured 1.0685x encode and 0.9311x decode. The effect may be binary layout,
instruction-cache interaction in the per-frame encode/decode loop, or host
frequency behavior, but the measured complete binary is authoritative.

Artifacts:

- `artifacts/exp0114-rice-grouped-confirm.tsv`
  (`fe520af3f28a5628e0768079fba6e1be153e334d1742b9baf76c15519b184173`);
- `artifacts/exp0114-rice-grouped-noinline.tsv`
  (`df5abd8c8c9d829722b826430f5e9feb8a775c45d1a372f94b9cc66b7dfa9f9d`).

## Decision

Reject and restore EXP-0113's scalar lane emitter. A 6.65% encode gain does
not justify a reproducible 6.19% geometric decode loss for an intermediate
codec.

Retain the balanced arbitrary-binary confirmation harness. Before revisiting
grouped emission, measure separate encode-only and decode-only processes with
stable binaries and inspect text layout/I-cache counters. The literature's
tree bitstream packer remains relevant to CUDA/FPGA, but this particular
scalar reuse is not a CPU promotion.

## References

- [Research 0039](../research/0039-parallel-rice-bitstream-hardware.md)
- [EXP-0094](EXP-0094-rice4-emission.md)
- [EXP-0113](EXP-0113-parallel-rice-early-termination.md)
