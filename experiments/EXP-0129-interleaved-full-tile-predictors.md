# EXP-0129 — Interleaved full-tile predictors

Status: **ACCEPTED**

## Classification

**Version-5 CPU scheduling exploration** — expose instruction-level
parallelism between independent causal predictor chains without changing the
bounded-shard format intended for CUDA.

## Hypothesis

EXP-0127 attributes 31.50% of version-5 encode cycles to full-tile predictor
and residual staging. Each clamp-gradient tile is causal, but adjacent tiles
are independent. EXP-0101 established that alternating two independent
version-2 Rice tile chains can hide dependency latency on this CPU.

For one-thread version-5 encoding, advancing two same-shape, same-plane tiles
in lockstep should give LLVM independent loads, quantization, reconstruction,
and stores to schedule. It should improve geometric whole-codec encode
throughput by at least 1.05x over EXP-0126, retain at least 0.95x decode
throughput, and preserve every stream byte.

## Modification

- factor full-tile residual staging from shard encoding;
- add a paired staging kernel with separate reconstructed rows, left values,
  and folded buffers for two tiles;
- use it for adjacent same-plane, same-shape tiles when `threads == 1`;
- fall back to the unchanged scalar tile path at plane, width, height, or
  remainder boundaries;
- encode both folded streams independently with the accepted EXP-0126 shard
  selector and direct-lane emitter.

Do not change prediction, quantization, residual order, entropy decisions,
syntax, tile geometry, thread spawning, or output assembly. The paired kernel
is a CPU scheduling analogue of assigning independent causal chains to
separate GPU warps; it introduces no cross-tile dependency.

## Test

- add paired-versus-scalar folded-residual equivalence tests covering full,
  edge, and multi-row tiles at lossy and exact quality;
- retain the version-5 control SHA-256 and every native q90 byte/metric;
- compare a fixed candidate binary with the exact EXP-0126 binary in five
  balanced trials across the four native 10/12/16-bit samples;
- require at least 1.05x geometric encode and 0.95x geometric decode;
- if accepted, profile the staging share and run the full release suite, both
  strict Clippy configurations, formatting, and diff checks;
- if rejected, revert the implementation and retain only this immutable
  record and its artifacts.

## Result

The q90/q100 unit oracle produces identical folded residuals and entropy
payloads for paired and scalar staging. The candidate also emits the accepted
version-5 HDR control SHA-256
`9a3cf708ecdc73f9f8c15a545b41f761ad1ed844c2b8cb4db42118ce587fce37`.
All four native q90 samples retain identical encoded bytes, compression ratio,
encoded-stream bitrate, PSNR components, luma block SSIM, and maximum error.

Five balanced whole-codec trials measured:

| Sample | Candidate encode | Encode ratio | Decode ratio | Encoded bitrate |
|---|---:|---:|---:|---:|
| HDR gradient 10 | 38.905 MP/s | 1.074x | 1.023x | 333.288000 Mb/s |
| Precision motion 10 | 41.972 MP/s | 1.100x | 0.976x | 148.023112 Mb/s |
| Precision UI 12 | 48.438 MP/s | 1.159x | 0.977x | 229.381632 Mb/s |
| Precision motion 16 | 63.065 MP/s | 1.212x | 0.978x | 38.988880 Mb/s |
| **Geometric** | — | **1.1349x** | **0.9884x** | — |

Every sample improves encode by at least 1.074x and retains at least 0.976x
decode, so the declared complete-binary gates pass. The exact EXP-0126 binary
has SHA-256
`739e68994d7a04c602967f8fee0d09d001821dd2551293c769c8d211e8d67f29`;
the fixed candidate binary has SHA-256
`fc8ba0d5444acaee395fad8e513f16556e77e7d984dfdfccfbce8f949bd03160`.

The profiling-feature candidate measured 1,497.89 ms task-clock, 5.318
billion cycles, 21.493 billion instructions, 3.304 billion branches, and
47.274 million branch misses over 30 HDR encodes: approximately 4.04
instructions/cycle and 41.53 MP/s. Cache references were available, while
cache misses again reported an unusable zero.

A 60-repeat cycle profile captured 12K samples with none lost:

| Stage/symbol | EXP-0129 self cycles | EXP-0127 |
|---|---:|---:|
| exact Rice selection plus fused zero-run cost | 39.92% | 36.21% |
| paired predictor/residual staging | 16.36% | — |
| scalar boundary-tile staging | 7.62% | — |
| all predictor/residual staging | 23.98% | 31.50% |
| shard emission/selection combined | 22.31% | 19.96% |
| fixed-block body emission | 4.90% | 4.08% |

Predictor staging falls 7.52 percentage points while the instrumented
end-to-end task clock improves 1.1196x over EXP-0127. The selector becomes the
clear next bottleneck because useful staging work became faster; the result
supports rather than merely correlates with the intended scheduling effect.

Artifacts:

- `artifacts/exp0129-interleaved-full-tile-confirm.tsv`
  (`35e3e63d00ed70fe1eb2432c87790abf5d46e8f0a2e4c67413c6e21e105a5cef`);
- `artifacts/exp0129-stage-perf-stat.tsv`
  (`280f97e13683209c352b154752a0b42032c0135aa7bb5577c04e543ce8a4f1b6`);
- `artifacts/exp0129-stage-perf.data`
  (`705882cbfd4591a87348b5236eb556223dd825188c0779e5f1d2274be649dda2`);
- `artifacts/exp0129-stage-perf-report.txt`
  (`2dc5cfd9465040ea82b04f8e72f3c2edbef069712efcc445e21a40507fb5e5de`).

All 70 library tests, motion/squeeze and binary targets, documentation tests,
normal and profiling-feature strict Clippy, shell syntax, formatting, and
diff checks pass.

## Decision

Accept interleaved version-5 full-tile predictor staging. It is a
byte-identical, broadly faster execution schedule and introduces neither
format overhead nor cross-tile dependency. It also gives the eventual CUDA
implementation a tested decomposition: assign independent causal chains to
independent execution contexts, then feed their residuals to bounded entropy
shards.

Version 5 remains a non-promoted low-serialization branch. Relative to the
fixed version-2 rows from EXP-0110, its geometric encoder advances from about
0.5810x to about 0.6594x. Exact Rice/zero-run selection is now the dominant
CPU target, but further exploitation should reduce arithmetic or residual
passes rather than repeat EXP-0128's control-flow-only specialization.

## References

- [Research 0036](../research/0036-independent-chain-software-pipelining.md)
- [Research 0040](../research/0040-edge-gpu-predictive-compression.md)
- [EXP-0101](EXP-0101-tile-pair-speed-promotion.md)
- [EXP-0126](EXP-0126-selector-fused-zero-run-cost.md)
- [EXP-0127](EXP-0127-post-zero-cost-profile.md)
