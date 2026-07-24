# EXP-0066 — Maximum-compression kernel profile

Status: **ACCEPTED**

## Classification

**Exploitation diagnosis** — select a SIMD/cache optimization target on the
current maximum-compression frontier after the broader partition and motion
exploration round.

## Hypothesis

On the version-3 maximum-compression tier, either order-0 rANS
encode/decode/table work or residual reconstruction will account for at least
20% of exclusive samples on a Rice-heavy camera workload. Generated assembly
and cache counters will distinguish an independent loop suitable for safe
layout/auto-vectorization work from an inherently serial entropy state
transition.

## Modification

No codec change. Profile:

- exact preserved maximum-compression binary
  `artifacts/frontier/fastvid-rans-exp0055`;
- a symbolized source reproduction of the same codec at current `master`,
  whose subsequent changes are benchmark-interface and model tooling only.

Use the 24-frame 1920x1080 noisy-camera clip, q90, GOP 1, and one thread after
an untimed warm-up. Record 999 Hz `perf` call graphs and five PMU trials for
cycles, instructions, branches, branch misses, L1D loads, and L1D load
misses. Use a no-LTO debuginfo build only for source attribution.

Inspect generated assembly or LLVM vectorization remarks for the leading
independent loop. Do not infer a SIMD opportunity solely from a scalar source
loop.

## Test and gate

- validate the preserved binary hash;
- require zero lost profile samples;
- reject unsupported zero-valued PMU aliases;
- require under 3% PMU trial spread;
- select an optimization only when a coherent kernel exceeds 20% exclusive
  samples or a small related set exceeds 30%;
- preserve whole-process counters as diagnostics, not codec throughput.

## References

- [Research 0012](../research/0012-simd-cache-profiling.md)
- [Research 0019](../research/0019-modern-integer-entropy-kernels.md)
- [EXP-0034](EXP-0034-perf-samply-cache-profile.md)
- [EXP-0055](EXP-0055-modeled-rans-selector.md)
- [EXP-0062](EXP-0062-speed-tier-entropy-profile.md)

## Result

The preserved EXP-0055 binary hash was
`dda826459cfa9cb017b751749d2b780419b18cc1a2ff9ff309492ea8b4df61da`.
Its 999 Hz profile collected 5,207 samples with zero lost samples. The
focused benchmark encoded 29,518,163 bytes (3.371917x) at 12.893 MP/s and
decoded at 40.864 MP/s.

The exact-binary exclusive profile was:

| Symbol | Samples |
|---|---:|
| encode closure | 72.41% |
| `decode_tile_payload` | 12.64% |
| `reconstruct_sample` | 10.14% |
| benchmark metrics | 1.19% |

A no-LTO debuginfo build was used only for source attribution. It separated
the encode closure into `encode_best_tile` (19.49%),
`ResidualAccumulator::push` (9.48%), spatial prediction (6.11%),
quantization (4.64%), Paeth (2.29%), varint-length modeling (2.10%), and
zero-run counting (1.78%). `best_rice_parameter` was only 0.77%.
Consequently, exhaustive causal predictor/residual evaluation is a coherent
35.08% hot set; it is not an independent data-parallel loop. Decode payload
handling plus reconstruction is a separate coherent 22.78% set.

Five exact-binary PMU trials averaged:

| Counter | Count | Relative spread |
|---|---:|---:|
| cycles | 17,582,366,787 | 0.25% |
| instructions | 55,571,263,698 | 0.00% |
| branches | 7,455,365,191 | 0.00% |
| branch misses | 155,611,186 | 0.02% |
| L2 data requests | 56,944,893 | 0.36% |
| system fills | 59,180,682 | 0.34% |
| L2 data read misses | 3,447,825 | 4.43% |

IPC was 3.16 and the branch-miss rate was 2.09%. The generic L1D-miss alias
returned zero on this AMD KVM guest and was rejected. The model-specific L2
read-miss subtype exceeded the 3% repeatability gate, so it is retained as an
inconclusive diagnostic and not used to claim a cache-miss rate.

LLVM vectorization remarks were mostly unattributed under thin LTO. Source
inspection confirmed that the leading predictor loops carry reconstructed
left/above state and the rANS loop carries its entropy state. Neither is a
sound target for explicit SIMD in the current one-state format. The existing
order-0 decoder does, however, allocate a complete folded-symbol vector and
then reread it during reconstruction. Giesen's reference decoder writes each
decoded symbol directly to its consumer buffer; Fastvid can likewise fuse
these two stages without changing bytes or introducing unsafe code.

Artifacts:

- exact profile: `artifacts/exp0066-maximum-perf.data`
  (`cf9d6d328752732b1ffcc4d61553d0f879948a1831b9986b42791cdedb782185`);
- source-attribution profile:
  `artifacts/exp0066-maximum-source-perf.data`
  (`1bfb6fe7d58d443e0fb369437e9f2b5b9649ff13f9ee32bee38d1e22eb879717`);
- core PMU summary: `artifacts/exp0066-maximum-core-stat.tsv`
  (`b23f77f4e26de306fbbc6249969a85b9a56a7e370d43d572562644385eb6b3a8`);
- cache PMU summary: `artifacts/exp0066-maximum-cache-stat.tsv`
  (`779f0058bdac9e7a7ee45aea9528e5e2e3116cac006e8d8a99a2eeab7798cbdb`).

## Decision

**Accepted as an actionable diagnosis.** Do not add explicit SIMD to the
serial predictor or one-state rANS loops. Test direct rANS-to-reconstruction
consumer fusion first; revisit interleaved SIMD only as an explicitly new
bitstream mode with enough independent states.
