# Corpus source policy

The canonical corpus is source-mastered. A normalized RGB PNG is a derivative,
not the reference source.

## Source preference

In descending order:

1. camera RAW, scene-linear OpenEXR, HDR/RGBE, or uncompressed high-bit TIFF;
2. raw 10/12/16-bit YUV or losslessly stored high-bit RGB;
3. lossless 8-bit PNG only where the content class is otherwise unavailable,
   notably screenshots captured from distinct open-source games;
4. JPEG only as a temporary discovery proxy, never as the high-bit reference.

External source masters are fetched into the gitignored `corpus/sources/v1/`
tree by the pinned setup script and verified byte-for-byte against committed
checksums. Only the frozen AI originals are distributed as a tarball. Network
retrieval is never part of an evaluation run.

## Independence and diversity

- At most two frames may come from one movie, clip, video sequence, or rendered
  animation.
- Samples from one sequence must be non-adjacent and visually distinct.
- At most one screenshot may come from one video-game title.
- At most one camera RAW sample may come from one camera model.
- At most one HDRI may come from one Poly Haven asset.
- Search-result order must never satisfy a category quota by itself. The
  manifest validator enforces source-group caps.
- AI images are generated once. Their exact original bytes, prompts, and source
  hashes are frozen; rebuilding must never invoke image generation.

## Derivatives

Derivatives must record the exact source hash, tool versions, command or
parameters, color interpretation, crop/resize policy, and output hash.

- Camera RAW is developed deterministically to 16-bit linear RGB.
- HDR material remains available in its original transfer function and bit
  depth. Any SDR view is explicitly color managed and is used only for SDR
  metric paths.
- Raw YUV is preserved without round-tripping through RGB.
- Chroma subsampling and bit-depth variants are produced only by the canonical
  evaluator's pinned conversion pipeline.

No 8-bit or tone-mapped derivative may stand in as the reference for a 10-bit
or 16-bit evaluation.
