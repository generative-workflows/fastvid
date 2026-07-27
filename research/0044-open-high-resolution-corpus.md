# Open high-resolution corpus sources

Date: 2026-07-27

Corpus v3 adds native-size inputs for GPU work while preserving the existing
v2 samples. Source files remain ignored artifacts; the manifest pins every
download by SHA-256.

## Selected material

- `Spring` (2048x858, 24 fps) and `Glass Half` (3840x2160, 24 fps) are
  CC-BY-4.0 Blender Foundation rendered controls. Their existing VP9 encodes
  are useful for scale and motion coverage, not pristine-source quality claims.
- `2019 People's Vote March` is uploader-owned real-world crowd footage,
  3840x2160 at 25 fps, by C.Suthorn under CC-BY-SA-4.0.
- `Calotes versicolor` is uploader-owned real-world animal footage,
  3840x2160 at 20 fps, by Aris riyanto under CC-BY-SA-4.0.

The two real-world WebM streams omit some or all colorimetry fields. The
reproducible codec-track conversion explicitly assumes limited-range BT.709,
resamples to 24 fps, converts 4:2:0 to planar 4:2:2, and retains native spatial
dimensions. This assumption is recorded in `corpus/manifest.json`; it avoids
claiming calibrated source color.

The reviewed 4K beach candidate was not promoted to the codec track because
it is HLG/BT.2020. It remains available for a future native HDR track rather
than being silently tone-mapped to SDR.

## Licensing boundary

The repository distributes fetch/conversion instructions, hashes, and
attribution. It does not commit the large media. Downloaded CC-BY-SA source
files and their derivatives retain CC-BY-SA-4.0 and are not relicensed under
the software license.
