# 0005 — Adaptive Rice residual coding

Sources:

- R. F. Rice and J. R. Plaunt, “Adaptive Variable-Length Coding for Efficient
  Compression of Spacecraft Television Data,” IEEE Transactions on
  Communication Technology, 1971. Public record:
  https://ntrs.nasa.gov/citations/19720033735
- M. Niedermayer, D. Rice, J. Martinez, “FFV1 Video Coding Format Versions 0,
  1, and 3,” IETF RFC 9043, 2021:
  https://www.rfc-editor.org/rfc/rfc9043.html
- M. van Beurden and A. Weaver, “Free Lossless Audio Codec,” IETF RFC 9639,
  2024: https://www.rfc-editor.org/rfc/rfc9639.html

Terms: the NASA record is publicly accessible US-government research. RFC
code components use the Simplified BSD License. Fastvid uses the algorithmic
ideas as literature and copies no implementation.

## Findings

Rice and Plaunt combine prediction with adaptive selection among simple
variable-length codes, reporting output close to difference entropy over a
wide entropy range. This establishes the useful pattern of selecting a code
per independently processed image block.

RFC 9043 specifies signed Golomb-Rice coding within independent FFV1 slices
and a distinct run mode. RFC 9639 explains the core mechanism directly:
zigzag-map signed predictor residuals, unary-code the quotient under a
power-of-two divisor, and store the fixed-width remainder. It also selects
Rice parameters over partitions because residual statistics vary.

## Fastvid implications

1. Keep the accepted zero-run syntax for sparse tiles.
2. Add Rice parameters 0 through 8 for tiles with dense small residuals.
3. Compute exact code lengths from the residual histogram and select the
   smallest mode per tile; do not rely on a global content heuristic.
4. Put the selector in the tile directory so adaptation preserves random
   access and has no per-payload byte overhead.
5. Bound decoded folded residuals to 510, require zero padding, and reject
   trailing data to keep the experimental stream canonical.
