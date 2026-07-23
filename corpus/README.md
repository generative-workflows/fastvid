# Fastvid core corpus

The core corpus is a small, reproducible 1920x1080 YUV422p8 set used for every
standard evaluation. Large media is not stored in Git. Run:

```sh
scripts/fetch-corpus.sh
scripts/benchmark-corpus.sh
```

`manifest.json` defines six still-image samples and three 24-frame video
samples. The external inputs are lossless PNG masters from the Blender Open
Movies *Big Buck Bunny* (CC BY 3.0) and *Elephants Dream* (CC BY 2.5). The
fetch script verifies each PNG against Xiph's upstream SHA-256 manifest before
performing one explicit BT.709 limited-range YUV422p8 conversion.

The generated corpus lives in `artifacts/corpus-v1/`. Its canonical raw files
are verified against `derived-checksums.sha256`. License/readme files and the
upstream checksum manifests are retained beside the generated media.

The canonical raw set is about 309 MiB; the retained verified PNG source cache
brings a complete local copy below 500 MiB. It is not claimed to represent all
camera, screen, HDR, bit-depth, or chroma content; those gaps belong in the
extended corpus described in `EVALUATION_METHODOLOGY.md`.
