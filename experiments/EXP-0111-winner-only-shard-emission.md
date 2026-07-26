# EXP-0111 — Winner-only shard emission

Status: **REJECTED**

## Classification

**Version-5 speed exploitation** — remove known allocation and redundant
encoding work without changing the accepted EXP-0110 stream.

## Hypothesis

Computing exact zero-run, four-lane Rice, and fixed-block byte costs before
emission, then allocating and writing only the winning body, should:

- preserve every version-5 output byte and all rate/quality/access results;
- eliminate loser-body allocations and Rice-parameter heap allocation;
- improve geometric q90 native high-bit version-5 encode throughput by at
  least 1.75x relative to EXP-0110's 11.907/11.901/12.084/13.005 MP/s;
- leave decode throughput within the 5% timing tolerance; and
- retain the complete correctness, malformed-stream, Clippy, formatting, and
  Lean gates.

## Modification

Replace construct-all candidate selection with exact integer byte-cost
functions. Use fixed four-element lane accumulators for Rice parameter
selection. Emit zero-run, Rice at the already selected parameter, or fixed
block only after the canonical winner is known. Preserve tie order and all
version-5 syntax.

## Gate

- the checksummed EXP-0110 control stream remains byte-identical;
- q90 native encoded bytes and quality remain identical;
- candidate-only fast feedback records encode/decode MP/s, raw MB/s, and
  encoded bitrate against the fixed EXP-0110 results;
- geometric encode speedup is at least 1.75x and decode is no more than 5%
  slower;
- full validation passes.

## Result

The exact-cost implementation preserves the EXP-0110 control stream byte for
byte (`9a3cf708…fce37`) and every q90 encoded byte, bitrate, quality metric,
and maximum error. Fixed four-element Rice cost accumulators remove the
parameter-selection heap allocation.

Two candidate-only trials were compared with the fixed EXP-0110 rows; version
2 and OpenAPV were intentionally not rerun:

| Sample | EXP-0111 encode | Encode ratio | Decode ratio |
|---|---:|---:|---:|
| HDR gradient 10 | 11.256 MP/s | 0.945x | 0.949x |
| Precision motion 10 | 11.495 MP/s | 0.966x | 0.954x |
| Precision UI 12 | 11.637 MP/s | 0.963x | 0.939x |
| Precision motion 16 | 12.463 MP/s | 0.958x | 0.979x |
| **Geometric** | — | **0.9581x** | **0.9550x** |

The 1.75x encode gate fails decisively. Exact winner selection adds separate
zero-run and fixed-block cost scans; those scans cost more than emitting the
small loser bodies they replace. Heap allocation was not the controlling
bottleneck at this shard size. The decode ratio remains barely inside the 5%
tolerance and is timing noise because decoder code is unchanged.

Artifact:

- `artifacts/exp0111-winner-only-shards.tsv`
  (`63d19c606a5bf8fb73803a8479850cfbf48ddd510c0a5c19fc06bbd26a3d3498`).

## Decision

Reject and restore EXP-0110's construct-all emitter. Retain the candidate-only
fixed-reference benchmark loop.

Profile the accepted encoder before attempting another allocation change.
The likely next leverage is reducing the 17 full Rice-parameter scans or
fusing cost accumulation with residual production, but that must be grounded
in samples rather than inferred from source shape alone.

## References

- [Research 0034](../research/0034-block-bitpacking-kernels.md)
- [Research 0037](../research/0037-parallel-hardware-friendly-codecs.md)
- [EXP-0110](EXP-0110-full-tile-bounded-shards.md)
