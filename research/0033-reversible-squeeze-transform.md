# 0033 — Reversible squeeze and lifting transforms

## Sources and terms

1. Sneyers et al., *The JPEG XL Image Coding System: History, Features,
   Coding Tools, Design Rationale, and Future*, 2025, CC BY-SA 4.0:
   <https://arxiv.org/abs/2506.05987>.
2. JPEG XL reference implementation, v0.11.2:
   [`enc_squeeze.cc`](https://github.com/libjxl/libjxl/blob/v0.11.2/lib/jxl/modular/transform/enc_squeeze.cc),
   [`squeeze.h`](https://github.com/libjxl/libjxl/blob/v0.11.2/lib/jxl/modular/transform/squeeze.h),
   and
   [`squeeze.cc`](https://github.com/libjxl/libjxl/blob/v0.11.2/lib/jxl/modular/transform/squeeze.cc).
3. libjxl v0.11.2
   [BSD-3-Clause license](https://github.com/libjxl/libjxl/blob/v0.11.2/LICENSE)
   and
   [additional patent grant](https://github.com/libjxl/libjxl/blob/v0.11.2/PATENTS).

The 2025 paper is a recent primary overview written by the codec authors. The
implementation is the more precise source for this transform. The reviewed
v0.11.2 files were retained locally while inspecting the algorithm; no code
has been copied into Fastvid.

libjxl is BSD-3-Clause and its distribution includes an additional patent
grant for claims necessarily infringed by that implementation. This is
compatible evidence for studying and independently implementing the reviewed
operation, but it is not a general patent-clearance opinion for arbitrary
extensions.

## The transform

JPEG XL Modular's squeeze is a Haar-like reversible lifting transform. For a
pair of integer samples `A, B`, its forward horizontal kernel forms:

```text
avg  = (A + B + (A > B ? 1 : 0)) >> 1
diff = A - B
detail = diff - SmoothTendency(left, avg, next_avg)
```

The average stays in the source range. The separate detail channel has a
wider theoretical range but is intended to concentrate energy around zero.
An odd final sample passes through to the average channel. Vertical squeeze
uses the same construction on adjacent rows. Repeated alternating
horizontal/vertical applications produce a low-resolution channel and
ordered detail bands.

The inverse recovers `diff` by adding the identical tendency and then:

```text
A = avg + diff / 2
B = A - diff
```

where signed integer division and the biased average are paired so every
integer input is reconstructed exactly.

`SmoothTendency` predicts the pair difference only in a monotonic
neighborhood and clamps the estimate to avoid overshoot/ringing. It therefore
adds causal dependency and branches to the otherwise simple pairwise lifting
step. A plain no-tendency Haar candidate is a useful lower-compute control:
it remains reversible and vectorizable, but may leave a broader detail
distribution.

## Implementation lessons

- The forward transform is linear-time, pair-local except for the tendency,
  and needs no global search. Its low/detail split is a qualitatively
  different coding branch from Fastvid's current search over several
  full-resolution predictors.
- A vertical transform naturally exposes independent columns to SIMD.
  libjxl's horizontal inverse has a leftward dependency, so its optimized
  implementation transposes groups of eight rows and treats them as vertical
  work. This is evidence that layout and dependency direction must be chosen
  together; merely translating scalar equations to intrinsics is
  insufficient.
- The transform does not reduce the number of source samples. Compression
  gains exist only if separately coding low and detail bands saves more than
  their extra mode, length, table/state, alignment, and directory costs.
- Tile-local application preserves Fastvid's independent tile access. It
  should not change default tile geometry, and per-tile fallback must charge
  the control bit/byte and retain current coding when the transform loses.
- For q100, reversibility can be proven directly over integer arithmetic. For
  q90, quantizing low/detail coefficients changes the error distribution and
  requires a new quality/error analysis; equal scalar quantizer steps do not
  imply Fastvid's current per-sample maximum-error bound.
- A fast intermediate codec should first screen plain horizontal, plain
  vertical, and one-level 2D lifting. Repeated pyramids and tendency search
  are higher-effort branches, not justified defaults.

## Fastvid synthesis

The first model should be q100-only and tile-local:

1. apply a one-level no-tendency reversible pair transform to original
   samples;
2. spatially predict the average and detail bands independently;
3. charge the actual current Rice/zero-run payload of both bands plus a mode
   byte and substream-length varint;
4. compare with each current tile's actual payload and retain exact fallback;
5. report selection rate, complete bytes, work passes, and results by content,
   plane, resolution, and bit depth.

This isolates whether frequency separation contains real byte savings before
format, SIMD, or lossy-quality complexity is introduced. A positive size
bound is not yet a speed result: the transform and second entropy stream add
work. Conversely, a simple vertical or pairwise kernel may become attractive
for a speed branch if it replaces rather than supplements multi-predictor
search.

## Experiments

- [EXP-0075](../experiments/EXP-0075-charged-reversible-squeeze-model.md):
  the charged one-level model was rejected at 0.801% aggregate payload
  savings. Its 2D candidate was useful mainly on one chroma plane and did not
  justify a format or compute cost.
- [EXP-0074](../experiments/EXP-0074-fixed-predictor-high-bit-speed.md):
  fixed-predictor high-bit speed branch establishes predictor
  search as the current high-bit encode bottleneck and supplies the compute
  budget against which a transform must be judged.
