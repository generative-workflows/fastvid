# OpenAPV application and parameter behavior

## Sources

Pinned Academy Software Foundation OpenAPV `v0.3.0.0`, BSD-3-Clause:

- [public preset definitions][presets];
- [encoder parameter defaults][defaults];
- [encoder application timing report][encoder-app];
- [decoder application timing report][decoder-app];
- [CMake SIMD source selection][cmake].

[presets]: https://github.com/AcademySoftwareFoundation/openapv/blob/v0.3.0.0/inc/oapv.h
[defaults]: https://github.com/AcademySoftwareFoundation/openapv/blob/v0.3.0.0/src/oapv_param.c
[encoder-app]: https://github.com/AcademySoftwareFoundation/openapv/blob/v0.3.0.0/app/oapv_app_enc.c
[decoder-app]: https://github.com/AcademySoftwareFoundation/openapv/blob/v0.3.0.0/app/oapv_app_dec.c
[cmake]: https://github.com/AcademySoftwareFoundation/openapv/blob/v0.3.0.0/src/CMakeLists.txt

## Findings from the code

`OAPV_PRESET_DEFAULT` is `medium`. The parameter initializer sets 256x256
tiles, 4:2:2 10-bit profile, limited range, and unspecified color
characteristics unless the caller overrides them. OpenAPV applications expose
explicit preset, QP, tile, thread, input-depth, and profile controls, allowing
those variables to be recorded rather than inferred.

The encoder application's `Total encoding time` is accumulated around codec
work and printed with millisecond precision. The decoder prints its accumulated
codec time as integer milliseconds. Full 24-frame sequences are therefore
preferable to short decoder timing probes.

The upstream build conditionally compiles architecture-specific kernels,
including x86 SSE4.1/AVX2 and ARM NEON paths. Comparison records must state
the actual build configuration and CPU rather than attributing results to the
portable algorithm alone.

The normative matched-comparison procedure belongs in
[`EVALUATION_METHODOLOGY.md`](../EVALUATION_METHODOLOGY.md), not in this
source-reference note.

## Relevant experiments

- [EXP-0031](../experiments/EXP-0031-openapv-matched-baseline.md)

