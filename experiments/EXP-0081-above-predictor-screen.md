# EXP-0081 — SIMD-friendly above predictor screen

Status: **REJECTED**

## Classification

**Speed exploration** — test a format-level predictor that removes the
horizontal reconstructed-sample dependency before investing in decoder or
SIMD implementation.

## Hypothesis

Predicting each sample from the reconstructed sample directly above permits
independent work across a row. On high-resolution natural and motion content,
its exact legacy zero-run/Rice payload may remain within 3% of the fixed
clamp-gradient speed branch while preserving the same quantization error
bound. If so, row-wide auto-vectorization or explicit SIMD could attack the
18.17% prediction kernel identified by EXP-0079.

## Model

Create a read-only high-bit model that:

1. tiles native planar input with the standard 256x128 geometry;
2. independently reconstructs clamp-gradient and above-only candidates using
   the production quantizer;
3. folds each residual and calculates exact zero-run and best-Rice payload
   lengths;
4. reports payload bytes, squared error, and maximum error for each candidate;
   and
5. never changes production stream syntax or codec source.

Run q90 and q100 on the matched 10-bit sequence and the complete native
high-bit supplement. Report complete modeled stream bytes by charging the
existing frame header and tile directory.

## Gate

Advance to a real predictor mode only when:

- the matched q90 complete-stream increase is at most 3%;
- aggregate complete-stream increase is at most 3% at q90 and q100;
- no corpus sample increases more than 10%;
- q90 squared/max error is no worse than fixed clamp-gradient;
- q100 remains exact; and
- the mode exposes a demonstrably dependency-free horizontal row kernel.

Failure is useful evidence that causal compression value exceeds the
available SIMD headroom.

## Result

The model's clamp-gradient control exactly reproduced EXP-0078's matched q90
stream: 18,882,860 complete bytes, including 69,888 bytes of per-frame header
and tile-directory overhead. It also reproduced the established maximum
error 4. This validates tile geometry, bit-depth-scaled quantization, causal
reconstruction, and exact legacy entropy charging at the primary operating
point.

The matched above-only result was 22,697,224 bytes, a **20.20%** increase.
Squared error changed from 293,341,442 to 293,305,442 (-0.012%) and maximum
error remained 4, so the failure is rate rather than quality.

Complete native-supplement results were:

| Sample | Quality | Clamp stream | Above stream | Change |
|---|---:|---:|---:|---:|
| 10-bit HDR gradient | 90 | 1,771,290 | 2,127,141 | +20.09% |
| 12-bit precision UI | 90 | 1,181,186 | 1,600,106 | +35.47% |
| 10-bit precision motion | 90 | 18,882,860 | 22,697,224 | +20.20% |
| 16-bit precision motion | 90 | 4,794,825 | 5,290,644 | +10.34% |
| 10-bit HDR gradient | 100 | 3,023,401 | 3,695,905 | +22.24% |
| 12-bit precision UI | 100 | 1,915,811 | 4,111,512 | +114.61% |
| 10-bit precision motion | 100 | 32,239,138 | 39,430,902 | +22.31% |
| 16-bit precision motion | 100 | 17,204,665 | 49,128,330 | +185.55% |

Aggregate complete-stream growth was 19.09% at q90 and 77.20% at q100.
Above-only q90 aggregate squared error was 0.057% lower and every maximum
error equaled clamp-gradient; q100 remained exact. The predictor therefore
meets the dependency and quality properties but fails every compression
gate. Its q90 penalty is also much larger than Fastvid speed's current 4.72%
bitrate advantage over matched OpenAPV `fastest`.

The read-only model passed strict Clippy and formatting. It remains in the
tree as a fast format screen; no decoder mode, syntax, or production codec
source changed.

Artifacts:

- complete model matrix:
  `artifacts/exp0081-above-model.tsv`
  (`11836372187ef0990d7e0144f1e7522f773c5dc1415c041de9bbc834f6855489`);
- release model binary:
  `target/release/above_model`
  (`45e4a82242ff4c26c4ae7920e9e9692278aa93cfcd7030e1064e8308fecf020f`);
- model source:
  `src/bin/above_model.rs`
  (`31b61cf7b2a09106c54e750f17db4bc70e8b59b98abdf100787d9c71e8d5203f`);
- corpus harness:
  `scripts/benchmark-above-model.sh`
  (`6a4737bc4bd1194824bb234604708df18a235b3600ad3d4ebfbea11c72fbbe2f`).

## Decision

Reject an above-only predictor as a standalone speed-frontier mode. Removing
horizontal causality is algorithmically attractive, but the lost
left/gradient information costs 10--186% on the current native supplement.
No plausible optimization of an 18.17% whole-command kernel can repay that
rate loss while preserving the declared matched boundary.

Retain the model for future compound predictors or transform screens. Do not
implement syntax or SIMD for this mode. The next speed direction must preserve
clamp-gradient's causal information, exploit parallelism across independent
tiles/planes, or batch entropy work without expanding the stream.

## References

- [Research 0012](../research/0012-simd-cache-profiling.md)
- [EXP-0074](EXP-0074-fixed-predictor-high-bit-speed.md)
- [EXP-0077](EXP-0077-high-bit-prefix-rice-streaming.md)
- [EXP-0079](EXP-0079-unified-speed-profile.md)
- [EXP-0080](EXP-0080-inlined-rice-writer.md)
