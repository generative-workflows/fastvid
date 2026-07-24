# EXP-0070 — Promoted four-state rANS profile

Status: **REJECTED**

## Classification

**Exploitation diagnosis** — measure the exact promoted maximum-compression
binary before introducing architecture-specific SIMD.

## Hypothesis

After EXP-0068's scalar batching, order-0 payload decode will still account
for at least 10% of exclusive whole-benchmark samples on the q90 all-intra
noisy-camera workload. If it falls below 10%, explicit SIMD cannot reasonably
clear a 5% whole-decode gate and should be deferred.

## Modification

No codec change. Profile the exact preserved EXP-0068 binary at 999 Hz with
DWARF call graphs on the 24-frame 1920x1080 noisy-camera clip, q90, GOP 1,
one thread. Record zero lost samples, exact binary hash, and benchmark rate,
throughput, and bitrate. Inspect generated assembly for vector integer
multiply, variable shift, or gather instructions.

## Test and gate

- preserved hash matches `frontier.json`;
- zero lost samples;
- a coherent entropy kernel accounts for at least 10% exclusive samples;
- do not count causal reconstruction as SIMD-addressable entropy work;
- select explicit SIMD only if assembly confirms the scalar baseline has not
  already vectorized the four-state batch.

## References

- [Research 0012](../research/0012-simd-cache-profiling.md)
- [Research 0030](../research/0030-entropy-decode-consumer-fusion.md)
- [EXP-0066](EXP-0066-maximum-compression-profile.md)
- [EXP-0068](EXP-0068-four-state-rans.md)

## Result

The preserved binary hash matched
`d4d7edaf68a67601f753652757d62bcc49ff237e9ef0954ad0174ddc45322a14`.
The profile collected 4,976 samples with zero lost samples:

| Exclusive symbol | Samples |
|---|---:|
| encode closure | 72.09% |
| `reconstruct_sample` | 10.58% |
| `decode_tile_payload` | 9.71% |
| block SSIM | 1.29% |
| plane comparison | 0.93% |
| rANS plan construction | 0.63% |

The focused run encoded 29,564,701 bytes (3.366609x) at 13.440 MP/s,
decoded at 46.516 MP/s, and produced 236.517608 Mb/s at the encoded-stream
boundary. Assembly inspection found no vector integer multiply, variable
shift, or gather instruction in the codec.

The entropy symbol missed the predeclared 10% whole-benchmark gate by 0.29
percentage points. This sampling view mixes encode and decode wall time, so
it does not prove entropy is unimportant inside a decode-only interval.
However, it does reject this experiment's stated evidence threshold for
introducing unsafe architecture intrinsics.

The subsequent code review in research 0031 strengthens the deferral:
htscodecs obtains AVX2/AVX-512 utilization with 32 states, not four, and now
defaults to simulated scalar gathers because hardware gather is slow on Zen 4
and on Intel systems with Gather Data Sampling mitigations. A credible SIMD
follow-up therefore needs a wider-state byte model and decode-only kernel
measurement, rather than applying intrinsics to the current four-state loop.

Artifacts:

- exact profile: `artifacts/exp0070-promoted-rans-perf.data`
  (`95bda8cbd23aa3910b3a60abd33d7f30ad3d9f10e698ddb31520eac974c3924e`);
- benchmark row: `artifacts/exp0070-promoted-rans-benchmark.tsv`
  (`c2f8a819d167d33a6f24f75110f6317bbd3c9dd484b000923db89a0370e53f70`).

## Decision

**Rejected as an explicit-SIMD trigger.** Keep the safe scalar four-state
frontier. Revisit SIMD only after an eight/16/32-state complete-byte model and
a decode-only kernel benchmark show enough headroom to justify runtime
dispatch and a minimal unsafe module.
