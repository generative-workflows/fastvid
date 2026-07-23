# EXP-0026 — Native high-bit-depth foundation

Status: **ACCEPTED**

## Hypothesis

Separate `u16` frame types plus bitstream version 1 can add native
10/12/16-bit Gray and YUV 4:2:2 support without widening or slowing the
optimized 8-bit frame representation.

## Modification

1. Add explicit high-bit pixel formats, `Plane16`, and `Frame16`.
2. Specify version-one bit-depth signaling and little-endian raw packing.
3. Widen high-bit predictor/residual/entropy arithmetic to `i32`/`u32`.
4. Scale lossy quantization by bit depth while keeping quality 100 step one.
5. Preserve the specialized v0/8-bit path and v0 decoding.
6. Add high-bit encode/decode, CLI, metrics, and corpus benchmark paths.

## Test

- Exhaustively verify range formulas and quantizer bounds for 8/10/12/16 bits.
- Exact odd-dimension Gray and YUV 4:2:2 round trips at quality 100 for
  10/12/16 bits.
- Bound lossy error by half the bit-depth-scaled step.
- Reject samples with nonzero unused bits and references with mismatched depth.
- Round-trip zero-run and Rice extremes through the widened entropy coder.
- Decode existing version-zero fixtures and rerun the 8-bit regression matrix.
- Benchmark native 10-bit HDR and derived 12/16-bit diagnostics.

## Acceptance criteria

- All four bit depths have exact quality-100 conformance coverage.
- 8-bit encoded bytes and balanced performance remain unchanged within the
  established noise gate.
- No high-bit input is downshifted or tone-mapped.
- Malformed high-bit streams fail without panic or excessive allocation.
- Specification, Rust tests, Lean range proofs, and experiment results agree.

## Results

The implementation adds a separate `u16` frame and codec path for native
10/12/16-bit Gray and YUV 4:2:2. Version 1 signals layout and bit depth,
supports Rice parameters 0…16, validates widened residual bounds before
reconstruction, and rejects frame/tile allocations above the existing 1 GiB
resource ceiling. Version 0 and its `u8` hot path remain separate.

Release tests cover exact odd-size round trips for both layouts at 10, 12, and
16 bits; quality-100 8-bit coverage remains in the v0 suite. Additional tests
cover lossy bounds at all high depths, temporal references and depth mismatch,
Rice residual 131070 at every parameter, malformed depth/layout/directory
fields, high-bit metric peaks, and oversized dimensions. The complete result
is 24 passing release tests, strict Clippy with warnings denied, clean
formatting, and successful Lean proofs for quality-100 step one and the
10/12/16-bit folded maxima.

The checksummed native smoke corpus produced these single-trial development
measurements:

| Sample | Depth | Quality/GOP | Threads | Ratio | Encode | Decode | Raw encode/decode |
|---|---:|---:|---:|---:|---:|---:|---:|
| HDR gradient 1080p | 10 | 100/1 | 1 | 2.397x | 20.61 MP/s | 42.29 MP/s | 82.44 / 169.17 MB/s |
| Precision UI 1080p | 12 | 100/1 | 1 | 2.402x | 20.11 MP/s | 38.88 MP/s | 80.45 / 155.53 MB/s |
| Precision motion 720p/24f | 16 | 90/1 | 1 | 24.325x | 34.54 MP/s | 63.35 MP/s | 138.16 / 253.41 MB/s |
| Precision motion 720p/24f | 16 | 90/12 | 1 | 20.547x | 54.93 MP/s | 231.89 MP/s | 219.70 / 927.55 MB/s |
| HDR gradient 1080p | 10 | 100/1 | 4 | 2.397x | 49.56 MP/s | 81.52 MP/s | 198.26 / 326.09 MB/s |
| Precision motion 720p/24f | 16 | 90/12 | 4 | 20.547x | 132.89 MP/s | 303.98 MP/s | 531.55 / 1215.93 MB/s |

Quality 100 was byte-exact. The 16-bit q90 rows had maximum error 256,
52.91…52.93 dB plane PSNR, and 0.99495 luma block SSIM. The GOP-12 stream was
4.306 MB/s (34.446 Mb/s) at its 24 fps playback rate. The command-line
encode/decode path also reproduced the 10-bit corpus input byte-for-byte.

The five-trial 8-bit fast-feedback geomean was 77.830 MP/s encode and
102.438 MP/s decode, versus 77.480 and 102.504 MP/s for the preceding accepted
binary. That is +0.45% encode and -0.06% decode, inside the 5% noise gate, with
identical encoded sizes in all four cases.

## Conclusion

Accepted. Fastvid now preserves and codes native 10/12/16-bit samples without
downshifting, while retaining the specialized 8-bit representation and
performance. The high-bit corpus is an initial numerical/performance smoke
set; broader calibrated natural and production HDR content is still needed
before comparative quality claims.

## References

- [Research 0013](../research/0013-high-bit-depth-codec-design.md)
- [Format version 1](../specs/format-v1.md)
- [OpenAPV research](../research/0011-openapv.md)
