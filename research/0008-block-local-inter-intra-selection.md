# Research 0008 — Block-local inter/intra prediction selection

Status: **REVIEWED**

## Sources

- J. Bankoski et al., [RFC 6386: VP8 Data Format and Decoding
  Guide](https://www.rfc-editor.org/rfc/rfc6386), 2011, especially Sections
  3, 4, 16, and 18.
- Alliance for Open Media, [AV1 Bitstream & Decoding Process
  Specification](https://aomediacodec.github.io/av1-spec/), sections on mode
  information and intra/inter prediction.

Both are openly readable format specifications. They are used here only to
ground the general architecture of block-local prediction selection. Fastvid
does not copy their transforms, entropy syntax, interpolation, motion search,
or patented tools. Inclusion in this note is not a patent-clearance finding.

## Findings

- Interframes do not require every region to use temporal prediction. Mature
  block codecs carry prediction information per block and can reconstruct
  intra- and inter-predicted regions in the same frame.
- Prediction must use reconstructed reference pixels, not original input
  pixels, to keep encoder and decoder state identical through lossy chains.
- Local mode decisions isolate motion or scene changes: stable regions can
  exploit temporal redundancy while changed regions retain spatial
  prediction.
- A decoder only needs the signaled mode and its existing reference. Encoder
  heuristics therefore remain outside the normative bitstream and can evolve
  without a format revision.
- Fastvid's 256x128 luma tiles are much coarser than modern codec prediction
  blocks. Tile-local selection is nevertheless a low-complexity step that
  preserves independent tile payloads and parallel processing.

## Fastvid implications

The current directory already stores a prediction mode for every plane tile,
but the encoder's mean-luma-difference gate chooses one mode for the entire
frame. Move the activity decision to each tile and evaluate its own plane
samples against the co-located reconstructed reference. This can retain
temporal prediction in static areas of otherwise active frames and avoid
temporal residual expansion in locally changed areas.

Start with the existing mean absolute difference threshold of five so the
experiment isolates granularity rather than threshold tuning. Compute the
decision in a prepass, then run only the chosen predictor; exhaustively
encoding both modes would guarantee the smaller payload but would conflict
with the throughput goal.

## Relevant experiments

- [EXP-0005](../experiments/EXP-0005-gated-temporal-prediction.md) established
  the frame-level temporal baseline and identified the need for a more local,
  entropy-aware gate.
- [EXP-0006](../experiments/EXP-0006-tile-local-temporal-gating.md) tests
  tile-local mean-absolute-difference selection.
