# Paeth data-dependency kernel and fixed-predictor fast paths

## Question

EXP-0058 attributes about 18% of balanced encode samples to Paeth selection and
its absolute-distance work. Can an open implementation remove that work while
preserving the predictor exactly, and do production codecs support a separate
fixed-predictor speed path?

## Open sources

- `stb_image` at commit
  [`31c1ad37456438565541f4919958214b6e762fb4`](https://github.com/nothings/stb/blob/31c1ad37456438565541f4919958214b6e762fb4/stb_image.h),
  dual-licensed public domain or MIT.
- libjxl,
  [modular encode-effort documentation](https://github.com/libjxl/libjxl/blob/main/doc/encode_effort.md)
  and
  [fixed gradient encoder path](https://github.com/libjxl/libjxl/blob/main/lib/jxl/modular/encoding/enc_encoding.cc),
  BSD-3-Clause.
- W3C,
  [PNG filter algorithms](https://www.w3.org/TR/png-3/#9Filter-algorithms),
  the normative Paeth definition.

Only the small integer identity below is used. No external source code or
dependency enters Fastvid.

## Equivalent Paeth decision

For left `a`, above `b`, and upper-left `c`, the reference Paeth predictor
chooses the argument nearest `p = a + b - c`, with ties ordered `a`, `b`, `c`.
`stb_image` rewrites that decision around:

```text
threshold = 3*c - (a + b)
lo = min(a, b)
hi = max(a, b)
candidate = lo if hi <= threshold else c
result = hi if threshold <= lo else candidate
```

The source notes that this form has favorable data dependencies and permits
straightforward branch-free code generation. It removes three absolute values
and exposes min/max/select operations to the compiler. The decision remains
defined entirely in signed integers, so it applies unchanged to Fastvid's
8-bit and 10/12/16-bit sample domains; `i32` safely contains the full
16-bit threshold range.

This is not a lookup-table candidate. A Paeth choice table indexed by
`a-c` and `b-c` would require 261,121 entries even for 8-bit samples, add an
input-dependent cache access, and not generalize compactly to 16-bit. The
algebraic form directly targets the measured instruction cost without trading
it for cache pressure.

## Fixed-predictor speed tier

JPEG XL's documented lowest modular effort uses one fixed clamped-gradient
predictor, a fixed color transform, no learned meta-adaptive tree, and simpler
entropy tools. Higher efforts progressively add weighted predictors, trying
multiple predictors, learned trees, and more exhaustive search. Its reference
encoder also contains a dedicated loop for the fixed gradient case rather
than routing every pixel through the general predictor machinery.

This supports two separate Fastvid directions:

1. keep a byte-identical, fixed-Paeth kernel as the speed frontier;
2. make practical/max predictor exploration staged or effort-dependent rather
   than paying for every candidate on every tile.

The first is an exploitation experiment with no compression risk. The second
remains a later Pareto experiment because sampled selection can change bytes
and quality.

## Relevant experiments

- [EXP-0058: frontier speed profile](../experiments/EXP-0058-frontier-speed-profile.md)
- [EXP-0059: bounded Paeth kernel](../experiments/EXP-0059-bounded-paeth-kernel.md)
