# Native 10-, 12-, and 16-bit codec design

## Sources

- Academy Software Foundation, [OpenAPV][openapv], version `v0.3.0.0`,
  BSD-3-Clause.
- M. Niedermayer et al., [FFV1][ffv1], RFC 9043, 2021.

[openapv]: https://github.com/AcademySoftwareFoundation/openapv
[ffv1]: https://www.rfc-editor.org/rfc/rfc9043.html

## OpenAPV findings

OpenAPV makes bit depth an explicit frame/profile property rather than an
implicit storage detail. Its public API defines native little-endian 10- and
12-bit 4:2:2 formats, 10/12-bit 4:4:4 and 4:4:4:4 formats, and a 16-bit
4:4:4/4:4:4:4 profile. The frame syntax signals `bit_depth_minus8`; internal
transform and quantization shifts depend on bit depth. This supports the
important architectural conclusion: bit depth must be visible to format
validation, coding arithmetic, quality metrics, and comparison methodology.

OpenAPV's 16-bit profile uses a 12-bit internal transform path. Fastvid's
predictive residual design need not copy that compromise: lossless quality 100
can preserve all 16 input bits with signed 32-bit predictor/residual arithmetic.

## Fastvid range analysis

For unsigned samples of bit depth `b`:

- sample maximum is `2^b - 1`;
- signed prediction residual range is `[-(2^b-1), 2^b-1]`;
- maximum zigzag value is `2*(2^b-1)`;
- maximum zero-run nonzero token is `4*(2^b-1)-1`.

At 16 bits these are `65535`, `±65535`, `131070`, and `262139`.
Signed `i32` arithmetic and unsigned `u32` entropy values are sufficient.
The current `u16` folded residual, 511-bin histogram, and decoder limit 510
are not sufficient.

Rice parameters through 16 cover the widened range while keeping the maximum
quotient small. The selected parameter remains tile-local and syntax-compatible
with a one-byte mode field.

## Storage and API decision

Do not widen the existing `Plane<Vec<u8>>` to `Vec<u16>`:

- it would double 8-bit frame memory;
- it would perturb the optimized 8-bit cache behavior;
- it would force all callers to pay for a capability they may not use.

Add separate `Plane16`/`Frame16` types whose samples are host-native `u16`.
Raw interchange is tightly packed little-endian 16-bit words with unused high
bits required to be zero. Public encode/decode entry points are type-specific;
shared parsing and entropy primitives may be refactored internally.

Pixel format identifies layout and bit depth together:

- Gray8/YUV422p8;
- Gray10/YUV422p10;
- Gray12/YUV422p12;
- Gray16/YUV422p16.

This deliberately prepares grayscale mask/alpha coding as well as color.

## Quantization

Quality 100 must remain step one for every bit depth. For lower qualities,
scale only the lossy part of the 8-bit step:

`step(b, quality) = 1 + ((base_step(quality) - 1) << (b - 8))`

where `base_step = 1 + floor((100-quality)/5)`.

This keeps exact reconstruction at quality 100 and approximately preserves
quantization error relative to the full-scale signal at other qualities.
Rate-distortion experiments must still validate this choice; nominal quality
values are not comparable across unrelated codecs.

## Compatibility and versioning

Bitstream version 1 uses the formerly reserved header byte for
`bit_depth_minus8` and interprets the format byte as plane layout. A decoder
continues to accept version 0 as implicitly 8-bit. New encoders emit version 1.

The v0 8-bit entropy limits remain part of v0 validation. Version 1 validates
residual and Rice limits against signaled bit depth.

## Relevant experiments

- [EXP-0026](../experiments/EXP-0026-high-bit-depth-foundation.md)

