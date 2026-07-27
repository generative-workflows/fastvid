# EXP-0143 — CUDA warp-parallel Rice emission

Status: **ACCEPTED**

## Hypothesis

Mapping one warp to each of the four Rice lanes and scanning 32 symbols at a
time will reduce q90 emission by at least 30% and raise complete encoding above
3 GP/s without changing bytes.

## Modification

Retain selected lane byte lengths from exact analysis. During emission, use
four warps per Rice shard to compute exact code-bit prefix sums in 32-symbol
groups and atomically set disjoint logical stream bits. Zero-run and fixed-
block emission remain unchanged. Pad backing storage by three bytes so aligned
32-bit atomic OR operations at the final stream byte stay in bounds; return an
exact-length tensor view.

## Test

Run full byte-identity conformance, the real-world 4K q90/q100 benchmark, and
q90 stage profiling. Compare against EXP-0141.

## Result

Whole-stream conformance passed for 10-, 12-, and 16-bit inputs, q90 and q100,
odd edge tiles, and zero-run, Rice, and fixed-block shards. CUDA output was
byte-for-byte identical to the Rust v5 oracle.

On the 3840x2160 Calotes frame, complete-call q90 encoding improved from
2.957654 ms (2.804385 GP/s) to 2.196578 ms (3.776054 GP/s). Q100 measured
2.200503 ms (3.769319 GP/s). Q90 emission fell from 1.105 ms to 385.121 us, a
65.1% reduction; prediction was 1.040 ms and analysis was 374.593 us.

## Decision

Accept. The emission reduction exceeds the predeclared 30% gate, the complete
q90 call exceeds 3 GP/s, and all encoded bytes remain identical. Measure the
full corpus and the 1080p slice before choosing the next optimization because
the 4K result does not expose fixed per-call overhead.

## References

- [Research 0039](../research/0039-parallel-rice-bitstream-hardware.md)
- [Research 0042](../research/0042-gpu-variable-output-assembly.md)
- [EXP-0141](EXP-0141-cuda-parallel-entropy-analysis.md)
