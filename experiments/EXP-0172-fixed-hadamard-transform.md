# EXP-0172 — Fixed 8x8 Hadamard transform

Status: **REJECTED**

Date: 2026-07-29

Candidate revision: `5c793afff651acda240b2c8871ce4b5bb198ffa7`.
Baseline/evaluator revision: `2badceaf8644a830c1018da5cfda3c5776ebcd44`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

## Hypothesis

For q90, one tile-local 8x8 integer frequency transform with fixed
depth-scaled frequency quantization and deterministic q100 spatial fallback
will reduce canonical rejection bytes by at least 10%, retain every quality
gate, and pass all latency and throughput gates.

## Modification

For lossy tiles whose dimensions are divisible by eight, replace spatial
clamp-gradient residuals with block-major 8x8 Walsh-Hadamard coefficients.
Use eight warps per access tile, a 64-point two-dimensional-equivalent
Hadamard transform, DC quantum `step * 8`, and AC quantum `step * 12`.
Entropy-code the resulting zigzag coefficients with the existing exact
zero-run/Rice/block-pack/order-0 selection. The decoder applies the inverse
transform in the same block mapping. Edge tiles and every step-1/q100 tile
retain the spatial path, so existing exact reconstruction remains available.

This is one fixed transform-and-quantization mode. It does not add transform
size search, adaptive quantization, neural inference, or evaluator changes.

## Canonical commands

Both runs used the same settings:

```sh
PYTHONPATH=. python3 scripts/evaluate.py --tier rejection \
  --output ARTIFACT.json --libvship-revision v5.0.0 \
  --libvship-build '5.0.0 CUDA direct C API' \
  --libvship-gpu-id 0 --libvship-workers 2
```

Artifacts:

- baseline: `/tmp/fastvid-exp0172-block-transform-baseline-rejection.json`;
- candidate: `/tmp/fastvid-exp0172-hadamard8x8-candidate-rejection.json`.

The focused CUDA codec and evaluator suite passed: 26 tests.

## Result

The baseline passed all 11 samples with 347,833,953 bytes, 6.188000859x
compression, minimum SSIMULACRA2 93.697319, and maximum Butteraugli 0.084405.

The candidate failed before completing the tier:

- two YUV422-10 samples failed decode with CUDA entropy status 1 because
  transformed coefficient magnitudes exceeded the prototype's admitted
  folded-symbol bound;
- all three measured single-frame RGB latency cases exceeded the 0.5 ms
  decode gate, at 0.573232, 0.573664, and 0.515440 ms;
- the nine completed samples encoded to 267,382,103 bytes versus 218,700,247
  baseline bytes for the identical sample IDs, a **22.260% expansion**;
- completed-sample compression was 4.947814x;
- minimum SSIMULACRA2 was 91.699165 and maximum Butteraugli was 0.144163, so
  completed quality cases passed despite the rate and speed failures.

No full-tier run was permitted after correctness and speed failures. The
candidate's incomplete aggregate ratio is not compared to the 11-sample
baseline ratio; the overlapping-sample byte comparison above is authoritative.

## Conclusion

Reject. A raw DC plus uniformly coarsened Hadamard-AC representation expands
the actual entropy stream and makes RGB decode too slow. The experiment
