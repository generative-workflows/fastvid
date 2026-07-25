# EXP-0075 — Charged reversible squeeze model

Status: **REJECTED**

## Classification

**Compression exploration** — test a new reversible-transform branch rather
than another residual predictor, entropy-state widening, or global tile
constant. This follows research 0033 and is deliberately analysis-only.

## Hypothesis

A one-level tile-local integer Haar-like split can concentrate high-bit q100
content into separately predictable average/detail bands. After charging both
real Rice/zero-run payloads, entropy-mode control, a substream length, and
exact per-tile fallback, at least one horizontal, vertical, or 2D candidate
will reduce complete native high-bit payload bytes by 2% or more.

The transform must also expose enough savings to justify its extra pass.
Anything below 2% is rejected before format implementation regardless of
ideal entropy.

## Modification

Add a read-only `squeeze_model` executable. For every q100 tile in the native
high-bit supplement:

- form a reversible biased integer average and difference with no smooth
  tendency;
- evaluate horizontal, vertical, and horizontal-then-vertical one-level
  splits;
- Paeth-predict each signed band independently;
- apply the current zigzag mapping and exact current Rice/zero-run cost
  selection to each band;
- charge one transform/entropy control byte plus a canonical varint containing
  the first substream length for each split;
- retain the current encoded payload when every transform is larger.

The 2D candidate has three detail bands and one low band. It charges a control
byte and one canonical length for every substream except the last. Odd final
rows/columns pass into the average band. The model operates independently
inside the existing 256x128 tiles and changes neither source nor format.

## Test

1. Unit-test pair forward/inverse arithmetic over the complete 16-bit pair
   domain boundaries and representative signed detail extrema.
2. Require the model's current payload total to equal `analyze_entropy16`
   output for every encoded q100 frame.
3. Run all four checksummed native high-bit samples at GOP 1.
4. Report current bytes, best complete bytes, selection rate, winning
   transform, and savings by sample, plane, and bit depth.
5. Run the model twice and require byte-identical TSV output.

## Gate

- q100 transform inverse is exact for every tested tile;
- all syntax and substream overhead described above is charged;
- aggregate best-fallback payload savings at least 2%;
- no sample or plane expands after exact fallback;
- at least two content families and two bit depths select a transform;
- no production format or encoder change before the model passes.

Even a passing size result does not establish a speed improvement. A successor
must compare transform work against the EXP-0074 fixed-predictor branch and
the measured 80.724 MP/s OpenAPV `fastest` target.

## Result

The model evaluated 4,752 tiles across every frame of the four-sample native
high-bit supplement. Runtime inverse checks exactly reconstructed every
horizontal, vertical, and 2D transformed tile. The pair oracle also covered
every unsigned 16-bit first value against boundary, midpoint, adjacent, and
identity second values, plus signed transformed extrema.

| Group | Tiles | Current bytes | Best fallback bytes | Savings | Selected |
|---|---:|---:|---:|---:|---:|
| 10-bit HDR gradient | 216 | 2,805,449 | 2,770,047 | 1.262% | 73 |
| 10-bit motion | 2,160 | 29,923,537 | 29,554,219 | 1.234% | 720 |
| 12-bit UI | 216 | 1,794,745 | 1,794,745 | 0.000% | 0 |
| 16-bit motion | 2,160 | 16,056,411 | 16,055,928 | 0.003% | 7 |
| **All** | **4,752** | **50,580,142** | **50,174,939** | **0.801%** | **800** |

The 2D candidate won 799 tiles, vertical won one, and horizontal won none.
The effect was concentrated in plane 2, where fallback saved 2.743%; plane 0
saved 0.002% and plane 1 saved nothing. Exact fallback ensured no sample or
plane expanded, but it cannot turn a narrow win into a general format tool.

The complete model and summary were rerun and reproduced byte-identical TSV
output. The candidate fails the 2% aggregate gate and the requirement for
meaningful selection across the content families: 12-bit UI selected no
transform, while 16-bit motion saved only 483 bytes.

Artifacts and provenance:

- raw model matrix:
  `artifacts/exp0075-squeeze-model.tsv`
  (`9f22e929827d819d2d6d3e77d2d281dbaa86d01cd75c4fc40e7c294068b386a3`);
- model source:
  `src/bin/squeeze_model.rs`
  (`59e577e8a437054c1eb41c756879eb4bcc81b3fc9a8bedf06b704b845efae8b7`);
- model release binary:
  `56abe3b6507e67e439c3afc8bed1d3d0588f2ea3116394a31be4cdeecc73679b`;
- benchmark harness:
  `da81c7140ebf6c93cef358111ed426626b06ad53e3127baf77b145aa6e8ba017`;
- summary validator:
  `d58979418c703fb1e5283d0aeaa2f4c0e436ac054ee9b4886e34df9e76781daa`.

## Decision

Reject before format implementation. A no-tendency one-level squeeze does
not save enough complete bytes to justify extra transform passes,
substreams, decoder paths, or SIMD work. Preserve research 0033 as a useful
design reference, but do not pursue repeated levels or smooth-tendency search
without new corpus-independent evidence: those variants add work in the
opposite direction from the OpenAPV encode-speed target.

## References

- [Research 0033](../research/0033-reversible-squeeze-transform.md)
- [Research 0024](../research/0024-finite-block-ans-entropy-models.md)
- [EXP-0053](EXP-0053-finite-block-order0-model.md)
- [EXP-0074](EXP-0074-fixed-predictor-high-bit-speed.md)
