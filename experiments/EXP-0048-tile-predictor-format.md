# EXP-0048 — Tile-local predictor format

Status: **REJECTED**

## Classification

**Exploitation of the compression frontier** — implement the accepted
EXP-0047 oracle result exactly before optimizing predictor selection cost.

## Hypothesis

A version-2 stream that signals Paeth, average, clamp-gradient,
half-gradient, or previous-frame prediction in each existing tile-directory
mode byte will realize at least 8% complete-stream savings on the combined
corpus while preserving q100 exactness, the quantizer error bound, tile
independence, GOP depth, directory length, and practical decode speed.

## Modification

- Preserve the EXP-0045 balanced implementation at source commit `156054c`
  and its recorded binary hash.
- Add stream version 2 for both 8-bit and high-bit formats. The candidate
  decoder continues to accept legacy v0 8-bit and v1 high-bit streams.
- Define prediction modes:
  - 0: Paeth;
  - 1: previous reconstructed frame;
  - 2: average of reconstructed left and above;
  - 3: WebP-compatible clamped `left + above - upper_left`;
  - 4: WebP-compatible half-gradient from the left/above average.
- At encode time, exactly encode every applicable candidate per tile and
  select the smallest payload. Break ties in favor of the legacy current
  choice, then lower squared error, then mode order.
- Use each selected candidate's reconstruction as the reference for the next
  coded frame.
- Keep entropy modes, quantization, tile geometry, directory length, and
  single-frame dependency rules unchanged.
- Specify version 2 and add Lean definitions/proofs for the new predictors and
  mode domain.

The exhaustive selector is intentionally a first compression-frontier
implementation. A following experiment must replace repeated encoding with a
cheap proxy or staged selector.

## Correctness tests

- Candidate output must match EXP-0047 oracle bytes and selected modes on
  representative intra and GOP sequences.
- Every mode round-trips exactly at q100 for 8/10/12/16-bit.
- Every mode respects the existing maximum-error bound in lossy coding.
- Legacy v0/v1 streams remain decodable and reject new modes unless version 2
  is signaled.
- Individual tile decode matches full-frame decode for every spatial mode.
- Malformed and out-of-domain mode values are rejected.
- Release tests, strict Clippy, formatting, and Lean pass.

## Measurement

1. Run exact-stream and focused one-frame/GOP controls.
2. Compare actual encoded sizes and quality with EXP-0047's oracle artifact.
3. Run the preserved-binary fast feedback loop to quantify the initial encode
   cost and decode effect.
4. Run the complete corpus size/quality matrix at qualities 60, 75, 90, 95,
   and 100, with 8/10/12/16-bit groups separate.
5. Run focused single-frame access on the standard videos.

## Acceptance and frontier gate

- Combined complete-stream bytes improve at least 8%.
- Eight-bit complete bytes improve at least 5%, and at least four of the six
  predeclared categories improve at least 1%.
- Actual output stays within 0.1% of the exact oracle model.
- q100 remains exact; no lossy quality/category SSE group regresses over 1%.
- Decode and single-frame access regress no more than 10%.
- Initial encode slowdown is fully reported. Up to 4x is permitted only for
  the compression frontier and creates a mandatory selector-optimization
  follow-up; worse than 4x rejects the implementation structure even if the
  format remains promising.

If accepted, add this distinct source/binary to the compression slot in
`FRONTIER.md`; do not replace the balanced EXP-0045 line.

## Result

The version-2 format and exact fused selector realized the EXP-0047 space
oracle:

- 8-bit complete bytes: 643,788,260 to 589,494,332, or 8.43% fewer;
- maximum-compression combined bytes: 761,576,255 to approximately
  659,840,158, or 13.36% fewer;
- 85 of 90 8-bit rows matched the oracle exactly, and the other five differed
  by at most 61 bytes due to equal-cost reconstruction/reference cascading;
- all eight q90/q100 high-bit rows matched the oracle byte for byte;
- q100 remained exact and the quantizer error bounds held.

The fused selector improved substantially over separately encoding every
candidate, but the initial implementation missed performance gates:

- 8-bit fast-feedback encode slowdown: 3.44x;
- initial high-bit geometric-mean encode slowdown: 4.19x;
- 16-bit motion q90 decode regression: 67.66%, caused by 1,500 Paeth tiles
  replacing temporal-copy tiles, 2,100 of 2,160 tiles using zero-run entropy,
  and the resulting serial spatial reconstruction chains.

Primary artifacts:

- `artifacts/exp0048-fused-fast-feedback.tsv`
  (`08743b5921b917deb22d884a7ac01d184d728d1af0034c7b51082931a09efac5`);
- `artifacts/exp0048-actual-8bit-stills.tsv`
  (`a07e5a00e34fcabfa041d6d5f8ea05275978fabc1ce7843658b230332b0c86d8`);
- `artifacts/exp0048-actual-8bit-video.tsv`
  (`7c069fe929ae05886b647f4006810bbb34a9f7c6dc0653be2f1349854cadd55f`);
- `artifacts/exp0048-highbit-stills-ab.tsv`
  (`c834a5ea7880cec8522be7a549396d94b0dca41dcfb47a2488017f862a789a1b`);
- `artifacts/exp0048-highbit-video-ab.tsv`
  (`cd74fc1de7c842820b82d7c58e0fd2a16fea6a6a91d970fe7ffe58b142d0612f`).

## Decision

Reject the exhaustive implementation as the production compression frontier
because it misses the declared high-bit encode and decode gates. Retain the
accepted version-2 format and predictor evidence. EXP-0051 stages the
high-bit search while preserving maximum-compression bytes, and EXP-0052
adds a 16-bit temporal guard that passes the practical decode/access and
combined-space gates.

## References

- [EXP-0047: compatible predictor oracle](EXP-0047-compatible-predictor-oracle.md)
- [Research 0023: forward-citation space
  review](../research/0023-forward-citation-space-savings.md)
- [Research 0008: block-local inter/intra
  selection](../research/0008-block-local-inter-intra-selection.md)
