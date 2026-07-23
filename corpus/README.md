# Fastvid standard corpus

Corpus v2 is a reproducible, multi-resolution set used for standard
evaluation. Large media is generated or fetched rather than stored in Git.
Run:

```sh
scripts/fetch-corpus.sh
scripts/benchmark-corpus.sh
scripts/benchmark-access-corpus.sh
```

`manifest.json` defines twelve codec-track stills and six 24-frame videos
across 360p, 720p, 1080p, and 4K. Content includes:

- Blender Open Movie animation and naturalistic rendered detail;
- CC0 lossless TIFF camera photography;
- explicitly public-domain camera video with deterministic added sensor noise;
- synthetic graphics, UI-style scrolling, chroma edges, grain, and cuts;
- a project-owned AI-generated image with committed prompt provenance;
- source-native HDR PQ and RGBA alpha capability diagnostics.

The generated corpus lives in `artifacts/corpus-v2/`. Codec-track derivatives
are planar BT.709 limited-range YUV422p8 and are verified against
`derived-checksums.sha256`. HDR and alpha assets retain native YUV444p10/PQ and
RGBA representations and are intentionally excluded from current headline
scores.

Corpus v1 remains reproducible through `manifest-v1.json` and
`derived-checksums-v1.sha256`; v2 does not mutate its samples. Source URLs,
licenses, hashes, dimensions, frame rates, generation code, and the AI prompt
are recorded alongside the manifest.
