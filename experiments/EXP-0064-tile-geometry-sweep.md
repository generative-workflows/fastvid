# EXP-0064 — Rectangular tile-geometry sweep

Status: **REJECTED**

## Classification

**Exploration: partition geometry** — test an unmeasured format parameter that
trades entropy efficiency and predictor continuity against cache footprint,
parallel work, and localized access.

## Hypothesis

The inherited 256x128 default is not simultaneously optimal for the current
version-3 maximum-compression path and the version-2 speed path. At least one
rectangular geometry will either:

- reduce complete encoded bytes by at least 2% without slowing encode or
  decode by more than 5%; or
- improve encode or decode throughput by at least 8% without increasing bytes
  by more than 1%.

At q100 decoded samples must remain exact. Below q100, geometry can change
causal reconstructed-neighbor predictions even with the same quantizer, so
quality must be measured rather than assumed invariant.

## Modification

Extend the four benchmark commands with an optional trailing
`TILE_WIDTH TILE_HEIGHT` pair and include both values in every output row.
Existing invocations retain `CodecOptions::default()` and remain
byte-for-byte compatible.

Add a serial fast-screen harness over these luma geometries:

`64x64`, `128x64`, `128x128`, `192x192`, `256x64`, `256x128`,
`256x256`, `256x384`, `256x512`, `384x256`, `512x128`, `512x256`,
`512x512`, `1024x128`, and `1024x256`.

Do not change the codec default or any frontier source during screening.

## Test

1. Prove legacy benchmark invocations still report 256x128 and produce the
   same encoded bytes and reconstruction as before.
2. Prove an explicit 256x128 invocation is byte- and quality-identical to an
   omitted geometry.
3. Run the standard four-case fast-feedback matrix serially at q90/q100,
   GOP 1/12, and its declared thread counts.
4. Report complete bytes, ratio, encode/decode MP/s and MB/s, stream bitrate,
   tile/mode counts, and all quality metrics by geometry and content.
5. Advance no more than two geometries to six alternating trials.
6. If a geometry clears the rate/speed gate, run full corpus, high-bit, and
   single-frame plus tile-level access confirmation before changing a default
   or frontier slot.

## Gate

- exact q100 reconstruction;
- worst per-content q90 luma PSNR loss no greater than 0.05 dB, luma block
  SSIM loss no greater than 0.0001, and unchanged maximum error;
- deterministic encoded bytes per geometry;
- either the 2% rate gate or 8% speed gate above;
- at least four luma tiles at 1920x1080, so the screen does not collapse the
  frame into one serial task;
- no promotion without explicit tile-access amplification evidence.

The benchmark-interface change may be accepted independently if its backward
compatibility controls pass. A geometry that misses the promotion gates is
rejected without changing defaults.

## References

- [Research 0028](../research/0028-tile-geometry-tradeoffs.md)
- [EXP-0057](EXP-0057-automated-pareto-frontier.md)
- [EXP-0061](EXP-0061-three-version-frontier.md)

## Results

The benchmark extension passed its compatibility controls. On the q90 camera
still, an omitted geometry and explicit 256x128 geometry both produced
1,496,869 bytes with identical quality and entropy-mode counts. The release
suite passed all 52 tests, and formatting and warning-clean Clippy passed.

The serial screen covered 15 geometries and four content classes. It exposed
strongly content-dependent behavior: narrower tiles helped the 4K synthetic
grid but badly inflated temporal UI and cut streams, while large tiles reduced
video control/table overhead but often increased camera bytes and reduced
decode throughput.

Two geometries advanced to six cyclic trials:

| Geometry | Complete bytes | Byte delta | Encode MP/s | Encode delta | Decode MP/s | Decode delta | Area |
|---|---:|---:|---:|---:|---:|---:|---:|
| 256x128 | 2,948,779 | baseline | 24.574825 | baseline | 95.387514 | baseline | 1.000x |
| 192x192 | 2,854,813 | -3.187% | 24.702974 | +0.521% | 97.169940 | +1.869% | 1.125x |
| 256x384 | 2,880,759 | -2.307% | 25.133096 | +2.272% | 92.878852 | -2.630% | 3.000x |

The 192x192 screen reduced bytes in every case, but by very different amounts:
camera 0.238%, 4K grid 5.831%, temporal UI 9.319%, and temporal cuts 12.961%.
Its q90 maximum error remained 1; worst luma block-SSIM change was
-0.00000155. q100 remained exact.

Those results do not establish a generally superior fixed default. The
relative value depends on the corpus's mixture of camera, graphics, animation,
motion, resolutions, and threading, as well as the current per-tile rANS table
cost. A proposed full-corpus confirmation was stopped after 129 data rows
because a corpus-tuned default was no longer considered a valid general
solution. The partial file is retained but is not promotion evidence.

## Artifacts

- 15-geometry screen: `artifacts/exp0064-tile-screen.tsv`
  (`0dbe599980062a03ccdaebb653297404d028b3ba4441486ae9fbc6c000b1376e`);
- screen summary: `artifacts/exp0064-tile-screen-summary.tsv`
  (`8fc9c8de08798f958cf850b79779d7b3b191dcbdbc36b7202d04ace8058a37c1`);
- six-trial shortlist: `artifacts/exp0064-tile-shortlist.tsv`
  (`92ee34d4fc72602e9e1b4c4233fa0be99707987b1d11e96e1e0d5014ef33ae43`);
- shortlist summary: `artifacts/exp0064-tile-shortlist-summary.tsv`
  (`d70a878532684fa4932855fd5af7abcb9b28a30db6c611f5ce11f2f1e8e63de4`);
- intentionally incomplete diversity pass:
  `artifacts/exp0064-full-corpus.tsv`
  (`6746f1b699cf8b8882c55af9550b12edabcd08f1541ffd54b928f5067caf5a85`).

## Decision

**Rejected as a fixed-default optimization.** Keep the 256x128 default and all
three frontier versions unchanged. Retain the rectangular benchmark override,
reported geometry columns, screen harnesses, and summarizer as reusable
evaluation infrastructure. A future geometry proposal must be an explicit
encoder-effort search or a content-independent rule validated on held-out
corpora; it must not bake this corpus's apparent winner into the codec.
