# EXP-0171 — libjxl Butteraugli semantics

Status: **ACCEPTED**

Date: 2026-07-29

## Problem

The direct Vship evaluator initialized Butteraugli with intensity `1.0`. Vship's
argument is a display intensity target in nits, not a neutral multiplier. This
made Butteraugli scores roughly an order of magnitude too small and placed them
on an implausible scale relative to SSIMULACRA2.

The evaluator also selected the maximum of Vship's configured p-norm, 3-norm,
and infinity norm instead of naming the canonical scalar directly.

## Validation

Source inspection established that:

- libjxl's Butteraugli CLI uses 80 nits for SDR input;
- its primary printed distance is the maximum distortion-map value;
- Vship exposes that primary value as `norminf`;
- Vship computes each frame independently; no temporal pooling occurs.

On `xiph-sintel-01000-yuv422-8`, q90 produced SSIMULACRA2 `73.3658`.
Butteraugli infinity norm was `0.3924` at 1 nit but `3.5218` at 80 nits. At q70,
the corresponding values were `0.6216` and `6.2927`. Repeating a frame on the
same handler produced identical scores, ruling out accumulated sequence state.

## Change

Initialize Vship Butteraugli with an 80-nit intensity target and auxiliary
p-norm 3. Use `norminf` explicitly as the canonical Butteraugli score. Record
all three choices in evaluator reports. The existing per-frame gate remains
Butteraugli infinity norm `<= 1.0`.

## Canonical result

```sh
PYTHONPATH=.:cuda python3 scripts/evaluate.py --tier rejection \
  --output /tmp/fastvid-libjxl-butteraugli-rejection.json \
  --libvship-revision v5.0.0 \
  --libvship-build '5.0.0 CUDA direct C API' \
  --libvship-gpu-id 0 --libvship-workers 2
```

The rejection tier passed all 11 samples:

- minimum SSIMULACRA2: `93.69731903076172`, unchanged;
- maximum Butteraugli: `0.8034377694129944`, previously `0.08440515398979187`;
- compression ratio: `6.188000859134071`, unchanged;
- 31 focused CUDA/evaluator/corpus tests passed.

## Decision

Accept the 80-nit infinity-norm methodology and supersede all one-nit
Butteraugli baselines. Scores from before this correction are not comparable to
new results. The passing rejection result is fast feedback only; the existing
full-tier quality failures still prevent codec acceptance.
