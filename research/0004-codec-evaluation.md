# 0004 — Codec evaluation methodology and test material

Sources:

- A. Filippov et al., “Video Codec Requirements and Evaluation
  Methodology,” IETF RFC 8761, 2020:
  https://www.rfc-editor.org/rfc/rfc8761.html
- Xiph.Org test media: https://media.xiph.org/video/derf/

## Findings

RFC 8761 evaluates multiple quality points per sequence and reports PSNR per
color plane plus luma MS-SSIM. It uses rate-distortion curves rather than a
single quality/bitrate observation. The related NETVC methodology prefers
lossless sources and reproducible public test sets.

Xiph hosts uncompressed YUV4MPEG sequences but warns that redistribution
terms differ by clip and must be checked in each accompanying copyright file.

## Fastvid implications

1. Synthetic fixtures can diagnose mechanisms but cannot substantiate general
   compression claims.
2. Establish a small regression corpus and a broader periodic corpus; record
   exact source hashes, provenance, and license terms.
3. Report Y, Cb, and Cr PSNR separately and add luma MS-SSIM.
4. Compare rate-distortion curves at several quality settings, not isolated
   ratios.
5. Do not add a clip to the repository merely because Xiph mirrors it; verify
   that clip's explicit terms first.

