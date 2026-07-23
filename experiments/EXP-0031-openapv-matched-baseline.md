# EXP-0031 — Matched OpenAPV baseline

Status: **ACCEPTED**

## Hypothesis

A same-input, native-10-bit, all-intra comparison can identify whether
Fastvid's next optimization priority is encode compute, decode compute, or
compression while avoiding nominal-quality and raw-format confounders.

## Modification

No codec change. Build pinned OpenAPV `v0.3.0.0` through its official CMake
project, sweep its QP controls at the `medium` and `fastest` presets, and
compare measured quality points with Fastvid q90 and q100. Use explicit
256x128 tiles, one/four threads, and the high-bit corpus-v2 10-bit motion
sequence.

## Test

Follow [research 0015](../research/0015-openapv-matched-comparison.md):

1. confirm all upstream OpenAPV tests pass;
2. calibrate rate-distortion with serial 24-frame runs;
3. choose measured nearest-PSNR rows;
4. warm every confirmation cell and record five serial trials;
5. report median codec time, MP/s, raw MB/s, encoded bitrate/size, ratio,
   PSNR, SSIM, and maximum error;
6. preserve commands, per-trial rows, build identity, and corpus checksum.

## Acceptance criteria

- Results use identical source bytes and sample precision.
- No nominal QP/quality equivalence is claimed.
- q100 is called matched only if both reconstructions are exact.
- Timing and rate/quality differences identify a concrete profiling target for
  the next Fastvid optimization experiment.

## Results

Pinned OpenAPV `v0.3.0.0` was configured as a static Release build with CMake
4.2.3 and GCC 15.2.0 (`-O3 -DNDEBUG`). The build included its SSE4.1 and AVX2
sources on the AVX2/AVX-512-capable AMD EPYC-Genoa host. All 16 upstream CTest
cases passed. Fastvid used Rust 1.97.1/LLVM 22.1.6 at repository revision
`597d4da487ccc9e1ec94a8054ee927ed0c1b6155` plus the recorded worktree
changes.

The corrected five-trial confirmation contains 80 rows. Selected medians:

| Codec/control | Threads | Ratio | Enc. MP/s | Dec. MP/s | Stream Mb/s | Y PSNR | SSIM |
|---|---:|---:|---:|---:|---:|---:|---:|
| Fastvid q90 | 1 | 4.432x | 29.60 | 52.68 | 159.70 | 52.002 | 0.993731 |
| OpenAPV medium/qp22 | 1 | 4.408x | 17.67 | 61.44 | 160.57 | 51.535 | 0.992916 |
| OpenAPV fastest/qp23 | 1 | 4.464x | 80.14 | 59.78 | 158.55 | 51.736 | 0.993294 |
| Fastvid q90 | 4 | 4.432x | 95.79 | 138.04 | 159.70 | 52.002 | 0.993731 |
| OpenAPV medium/qp22 | 4 | 4.408x | 62.31 | 136.53 | 160.57 | 51.535 | 0.992916 |
| OpenAPV fastest/qp23 | 4 | 4.464x | 218.99 | 133.24 | 158.55 | 51.736 | 0.993294 |

The selected OpenAPV rows are slightly lower quality than Fastvid q90:
medium/qp22 by 0.468 dB and fastest/qp23 by 0.267 dB. Their compressed rates
are nevertheless very close, making this a useful local rate-quality
neighborhood rather than an exact quality tie. At one thread, Fastvid encodes
67.5% faster than the upstream default `medium` preset, while OpenAPV
`fastest` encodes 2.71x as fast as Fastvid. Decode rates are much closer.

Fastvid q100 is exact at 295.36 Mb/s. OpenAPV QP0 is not exact: both presets
have maximum error 2 and about 74.8 dB Y-PSNR at roughly 360 Mb/s. Those rows
are reported as OpenAPV's measured high-fidelity boundary, not a q100 match.

The full corrected rows are in
`artifacts/exp0031-confirmation.tsv`. The harness is
[`scripts/benchmark-openapv.sh`](../scripts/benchmark-openapv.sh).

## Conclusion

Accepted as the first matched native-10-bit baseline. Fastvid is already
competitive with OpenAPV's default encoder and decoder at the q90
rate-quality neighborhood, but OpenAPV's `fastest` preset exposes a 2.7x
single-thread encode gap at nearly the same rate and quality. The next
optimization should therefore target Fastvid's high-bit residual-generation
working set and scalar spatial dependency path while preserving its bitstream.

## References

- [OpenAPV research](../research/0011-openapv.md)
- [Matched comparison protocol](../research/0015-openapv-matched-comparison.md)
- [Evaluation methodology](../EVALUATION_METHODOLOGY.md)
