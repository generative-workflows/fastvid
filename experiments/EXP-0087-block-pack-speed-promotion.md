# EXP-0087 — Block-pack speed-frontier promotion

Status: **ACCEPTED**

## Classification

**Slow-tier confirmation** — promote the accepted EXP-0086 scalar block-pack
candidate only if fresh balanced internal and matched-OpenAPV measurements
support the speed slot.

## Hypothesis

EXP-0086 improves q90 rate and decode throughput while keeping encode within
the frontier's 5% timing tolerance, so it should dominate EXP-0078 as the
speed artifact without changing the 8-bit fast-feedback position. It is not
expected to beat OpenAPV `fastest` encode yet.

## Test

1. Register source commit `23801ae`, the checksummed release binary, and
   EXP-0086 evidence in a candidate frontier manifest.
2. Run six balanced serial trials over the three Fastvid slots and regenerate
   the four-case internal summary/graph.
3. Run six trials of every declared Fastvid and OpenAPV matched control at
   one/four threads on the pinned 10-bit all-intra sequence.
4. Regenerate the matched summary/graph and report rate, quality, encode, and
   decode gaps without treating nominal controls as equivalent.
5. Promote only if hashes validate, stream bytes/quality are deterministic,
   and EXP-0086 remains within the speed-slot tolerance.

## Result

All binary hashes validated. The internal four-case screening aggregates were:

| Slot | Ratio | Encode | Decode | Encoded stream |
|---|---:|---:|---:|---:|
| Practical compression | 24.547776x | 28.956 MP/s | 135.023 MP/s | 37.455 Mb/s |
| Maximum compression | 33.588694x | 24.811 MP/s | 102.315 MP/s | 27.374 Mb/s |
| Speed | 13.353556x | 117.002 MP/s | 145.835 MP/s | 68.854 Mb/s |

The block-pack change is high-bit-only, so the speed slot retained its
expected distinct 8-bit position.

Fresh one-thread matched q90 results were:

| Codec | Control | Ratio | Encode | Decode | Bitrate | Y PSNR |
|---|---:|---:|---:|---:|---:|---:|
| Fastvid speed | q90 | 4.809339x | 65.475 MP/s | 69.177 MP/s | 147.169656 Mb/s | 52.001930 dB |
| Fastvid practical | q90 | 5.307903x | 16.528 MP/s | 58.915 MP/s | 133.346224 Mb/s | 52.002293 dB |
| Fastvid maximum | q90 | 5.307903x | 16.698 MP/s | 59.778 MP/s | 133.346224 Mb/s | 52.002293 dB |
| OpenAPV medium | QP22 | 4.408004x | 17.633 MP/s | 63.468 MP/s | 160.568984 Mb/s | 51.534665 dB |
| OpenAPV fastest | QP23 | 4.464067x | 81.182 MP/s | 63.471 MP/s | 158.552448 Mb/s | 51.735588 dB |

Relative to OpenAPV `fastest`, Fastvid uses 7.18% less bitrate at 0.266 dB
higher Y PSNR and decodes 8.99% faster. Fastvid encodes 19.35% more slowly,
equivalently OpenAPV encodes 23.99% faster. At four threads Fastvid encodes
14.35% more slowly and decodes 19.92% faster.

At the distinct q100 boundary Fastvid is exact at 2.743688x, 66.038 MP/s
encode, and 257.969880 Mb/s. OpenAPV `fastest` QP0 has maximum error two at
1.965097x, 63.200 MP/s, and 360.180000 Mb/s. Fastvid is 4.49% faster to
encode and uses 28.38% less bitrate there, but this is not the practical q90
match required by the project goal.

Artifacts:

- internal raw matrix:
  `artifacts/exp0086-frontier-fast-feedback.tsv`
  (`0eb727840853a455bab77ba52cfff11489eb6667ba0ac005fa6fbb08f3029d14`);
- internal graph:
  `benchmarks/frontier.svg`
  (`26ceef2921799233243d3220759ca43c0e8b2db838742d8543018d0a6c62d651`);
- internal summary:
  `benchmarks/frontier-summary.tsv`
  (`bd53ba12cdbf5ad6e76a6478e1d3c73d112f88c794b7e529222bb0342157921b`);
- matched raw matrix:
  `artifacts/exp0086-openapv-frontier.tsv`
  (`b48462cea78b6c51d8d8ae2e51dd0b640b8308f47265a4467facc4b0825405b0`);
- matched graph:
  `benchmarks/openapv-frontier.svg`
  (`29350621e6eb37872cbff6f0a6f2e2807d730634d3785b3bc53be135f7586929`);
- matched summary:
  `benchmarks/openapv-frontier-summary.tsv`
  (`52852d509a40e4b8c277292cc3343d09875c338558bc25bf5d521dd89fbfe58f`);
- OpenAPV encoder/decoder:
  `9a65f11bc2d9d0602b52639e539821426c443626880f4edeb1c057dae657cd9b` /
  `4db9c098f1d0cbd60b6614a166aab3dce81ad534ca01c80d1f17c3aae77e9553`.

## Decision

Promote EXP-0086 over EXP-0078 in the speed slot. It materially improves
q90 rate and decode performance while encode remains within the declared 5%
tolerance, and the complete access confirmation also passed. Preserve the
EXP-0078 binary as historical evidence.

The practical q90 OpenAPV encode target remains unmet by 19.35%. Continue
with a fast-feedback microbenchmark for the scalar block unpacker/packer,
then test portable word-at-a-time kernels before architecture-specific SIMD
and runtime dispatch. Use the slow matched suite only to confirm a candidate
that passes the focused gate.

## References

- [Research 0034](../research/0034-block-bitpacking-kernels.md)
- [EXP-0078](EXP-0078-unified-speed-frontier.md)
- [EXP-0086](EXP-0086-sampled-block-pack-format.md)
