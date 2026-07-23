# Matched OpenAPV comparison protocol

## Scope

This protocol turns [research 0011](0011-openapv.md) into a reproducible
comparison now that Fastvid has a native 10-bit 4:2:2 path. It uses OpenAPV
`v0.3.0.0` built by its upstream CMake project and the checksummed
`high-precision-motion-10` sequence from high-bit corpus v2.

The comparison is an all-intra engineering diagnostic, not a claim that the
formats have identical tools or rate controls. Fastvid quality values and
OpenAPV QP values are never treated as equivalent.

## Controlled configuration

- Input: the same planar `yuv422p10le` bytes, 1280x720, 24 frames, 24 fps.
- Coding track: all-intra; Fastvid GOP 1 and OpenAPV's intra-only format.
- Tile geometry: 256x128 for both codecs. OpenAPV's library default is
  256x256, while Fastvid's current default is 256x128, so the explicit setting
  prevents tile count from becoming an unreported variable.
- Threads: 1 and 4.
- OpenAPV presets: `medium`, the upstream default and reference-quality
  operating point; `fastest`, reported separately as a speed-frontier
  diagnostic.
- Fastvid qualities: at minimum q90 and q100.
- OpenAPV controls: sweep QP before selecting quality-matched rows.

The OpenAPV Release build must record compiler, enabled architecture sources,
test result, and exact version. Fastvid uses the checked-in release profile.
Only one CPU-bound measurement process runs at a time.

## Quality matching

Every decoded stream is scored by the same Fastvid metric implementation:
aggregate Y/Cb/Cr PSNR, mean per-frame luma 8x8 block SSIM, and maximum native
code-value error. A comparison row is quality-matched to a Fastvid row by:

1. bracketing the Fastvid Y-PSNR with adjacent OpenAPV QPs when possible;
2. choosing the measured OpenAPV point with minimum absolute Y-PSNR distance;
3. reporting the PSNR difference and both SSIM values rather than hiding the
   residual mismatch.

Fastvid q100 is mathematically lossless. If OpenAPV has no exact reconstruction
in the sweep, its highest-fidelity point is reported as a capability boundary,
not as a q100 quality match. No interpolated timing or bitrate is presented as
a measured row.

## Measurements

The encoder and decoder application's internal codec clocks exclude raw input
and output file I/O. Each cell receives one unrecorded warm-up followed by five
serial trials for confirmation. Report medians of:

- encode/decode milliseconds;
- full-resolution luma MP/s;
- raw decimal MB/s from actual input bytes;
- encoded bytes, raw/encoded ratio, and bits per luma pixel;
- encoded stream decimal MB/s and Mb/s at 24 fps;
- all quality metrics above.

OpenAPV's decoder clock has integer-millisecond resolution. The 24-frame input
is therefore the minimum standard unit; smaller smoke tests are not suitable
for reportable decoder timings.

## Interpretation limits

The two application front ends do not necessarily charge identical
orchestration work to their internal clocks. Results therefore describe the
current command-line implementations under matched media, precision, tiling,
thread count, and quality—not an isolated primitive benchmark. Profiles may
explain differences, but optimization decisions still require Fastvid A/B
experiments under the standard methodology.

## Relevant experiments

- [EXP-0031](../experiments/EXP-0031-openapv-matched-baseline.md) applies this
  protocol.

