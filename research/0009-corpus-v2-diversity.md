# Research 0009 — Corpus v2 content diversity and licensing

Status: **REVIEWED**

## Sources

- Wikimedia Commons,
  [`Pontegana2.tif`](https://commons.wikimedia.org/wiki/File:Pontegana2.tif),
  a 2742x4096 camera photograph released under CC0 1.0.
- Wikimedia Commons,
  [`Cane Cholla Las Cruces NM.tiff`](https://commons.wikimedia.org/wiki/File:Cane_Cholla_Las_Cruces_NM.tiff),
  a 3264x2448 camera photograph released under CC0 1.0.
- Xiph.Org, [`FourPeople`](https://media.xiph.org/video/derf/), explicitly
  labeled public domain and available as a 1280x720 camera clip.
- Xiph.Org, [Derf's Test Media
  Collection](https://media.xiph.org/video/derf/), including its per-sequence
  redistribution warning and source/license links.
- A. Filippov et al., [RFC 8761](https://www.rfc-editor.org/rfc/rfc8761),
  especially its separate camera, screen-casting, and mixed-content
  requirements.

## Findings

- The existing Fastvid corpus is entirely computer-rendered. Its organic
  detail is useful, but it does not represent sensor noise, lens blur,
  demosaicing texture, or photographic tonal distributions.
- Widely mirrored codec test sequences are not automatically suitable for an
  MIT-compatible project. Xiph explicitly warns that some items carry extra
  restrictions, so public hosting alone is insufficient provenance.
- The selected Wikimedia photographs have explicit CC0 grants and lossless
  TIFF originals large enough for a deterministic 1920x1080 crop/scale.
- Screen edges, chroma-only discontinuities, temporally independent grain,
  and hard scene cuts can be generated directly as canonical YUV422p8. This
  avoids external licensing and makes the exact sample algorithm reviewable.
- A source-native resolution ladder distinguishes fixed per-frame overhead
  from pixel throughput. HDR PQ and RGBA alpha assets must remain native
  capability probes until the codec supports their metadata and planes;
  flattening them would erase the property under test.
- AI imagery is a distinct content class with characteristic dense,
  non-camera texture. Exact source pixels and the full generation prompt must
  be retained because model generation is not deterministic.
- Procedural samples should supplement rather than replace natural sources:
  they are controlled stress tests, not evidence of broad visual quality.

## Fastvid implications

Create corpus v2 as a strict superset of v1. Preserve the v1 manifest and
checksums because core samples are immutable within a version. Add two CC0
lossless camera stills, a public-domain camera clip with controlled added
noise, a prompt-pinned AI still, and deterministic procedural samples for
screen scrolling, chroma edges, changing grain, scene cuts, HDR, alpha, and
multiple resolutions. Pin every external byte stream with SHA-256 and every
canonical derivative with a separate committed SHA-256 list.

## Relevant experiments

- [EXP-0003](../experiments/EXP-0003-regression-corpus.md) defines the original
  corpus and tracks remaining coverage gaps.
- [EXP-0008](../experiments/EXP-0008-corpus-v2-expansion.md) implements and
  validates the expanded corpus.
