# EXP-0068 — Four-state interleaved rANS

Status: **ACCEPTED**

## Classification

**Format exploration informed by exploitation profiling** — introduce the
minimum independent-state structure needed for entropy instruction-level
parallelism, while initially using scalar safe Rust so format value is
separated from explicit SIMD.

## Hypothesis

A four-state byte-rANS tile mode will improve focused maximum-tier decode
throughput by at least 8% while increasing focused encoded bytes by no more
than 0.5%. A scalar interleaved implementation must pass this gate before an
AVX2 or AVX-512 kernel is justified.

## Modification

Prototype a version-3 entropy mode with:

- the existing tile-local normalized table;
- four little-endian final states instead of one;
- residual sample `i` assigned to state `i mod 4`;
- reverse-raster encoding into one shared renormalization byte sequence; and
- raster-order decoding cycling through the four states.

Charge the additional 12 final-state bytes in predictor/entropy selection.
Use the mode only when its exact payload remains smaller than zero-run/Rice.
Retain scalar order-0 decoding for existing streams. Use const-generic state
count internally so the compiler sees fixed independent states; do not add
unsafe code or architecture intrinsics in this experiment.

## Test

1. Add sparse, dense, extreme-alphabet, truncation, trailing-byte, and
   noncanonical-final-state unit controls.
2. Confirm q100 exactness, q90 decoded error bounds, individual-tile access,
   and compatibility with existing version-3 streams.
3. Alternate at least six focused one-thread q90 GOP-1 trials against the
   preserved EXP-0055 maximum-compression binary.
4. If the focused gate passes, run the standard frontier corpus matrix and
   q90 GOP-12 access confirmation.

## Gate

- focused decode throughput at least +8%;
- focused encoded bytes no worse than +0.5%;
- focused encode throughput no worse than -5%;
- standard-matrix decode no worse than -1%;
- quality invariance and all malformed-stream checks pass;
- preserve independent tile access and one-frame GOP-1 access.

## References

- [Research 0024](../research/0024-finite-block-ans-entropy-models.md)
- [Research 0030](../research/0030-entropy-decode-consumer-fusion.md)
- [EXP-0055](EXP-0055-modeled-rans-selector.md)
- [EXP-0066](EXP-0066-maximum-compression-profile.md)
- [EXP-0067](EXP-0067-fused-rans-reconstruction.md)

## Result

The format and kernel passed, but only after the fast-feedback loop exposed
two distinct implementation and selection problems.

The first cyclic-array decoder was slower than scalar rANS. Explicitly
batching four table lookups and state advances still produced a six-trial
**−3.92%** decode regression. A 5,183-sample profile had zero lost samples
and attributed 13.22% to `decode_tile_payload`, slightly worse than the
12.64% scalar baseline in EXP-0066. Generated assembly contained no vector
multiply, shift, or gather instructions.

The problem was not absence of explicit SIMD. Four separately fallible
advance helpers prevented a compact independent batch. Computing all four
`u64` next states, checking their combined overflow condition once, and then
renormalizing lanes in byte-stream order changed the six-trial focused result
to:

| Variant | Encoded bytes | Encode MP/s | Decode MP/s |
|---|---:|---:|---:|
| scalar EXP-0055 | 29,518,163 | 13.376500 | 42.365500 |
| four-state | 29,572,832 | 13.704833 | 48.043500 |

That was **+13.40% decode**, **+2.45% encode**, and **+0.185% bytes** without
unsafe code or architecture intrinsics.

The first standard matrix then showed that paying 12 bytes on every selected
rANS tile expanded already tiny UI and cut streams by 1.68% and 1.76%.
The final selector therefore uses four states only when their 12 extra bytes
are at most five per mille of the modeled scalar rANS payload; otherwise it
retains mode 10. This is an explicit per-tile byte budget, not a
corpus-selected tile or block size.

The final focused confirmation averaged:

| Variant | Encoded bytes | Encode MP/s | Decode MP/s | Playback bitrate |
|---|---:|---:|---:|---:|
| scalar EXP-0055 | 29,518,163 | 13.448333 | 42.749000 | 236.145304 Mb/s |
| budgeted four-state | 29,564,701 | 13.708833 | 48.734833 | 236.517608 Mb/s |

This is **+14.00% decode**, **+1.94% encode**, and **+0.158% bytes**.

Six-trial per-case medians followed by geometric means on the standard
fast-feedback matrix were:

| Variant | Compression | Encode MP/s | Decode MP/s |
|---|---:|---:|---:|
| scalar EXP-0055 | 33.613405x | 25.060312 | 98.193994 |
| budgeted four-state | 33.588694x | 25.230063 | 103.404300 |

The candidate changes are **−0.074% compression ratio**, **+0.68% encode**,
and **+5.31% decode**. UI and cuts retained exactly the baseline bytes;
camera and grid paid 0.093% and 0.201% respectively and improved decode by
11.31% and 9.62%. Every reported quality value was invariant.

The q90 GOP-12 access confirmation contained 48 source/target cases and six
trials per variant. Per-case medians followed by geometric means showed:

| Metric | Scalar | Four-state | Change |
|---|---:|---:|---:|
| encoded bytes read | 1,018,472 | 1,019,589 | +0.110% |
| access time | 112.382 ms | 104.875 ms | −6.679% |
| useful throughput | 16.119 MP/s | 17.273 MP/s | +7.157% |
| work throughput | 58.032 MP/s | 62.186 MP/s | +7.157% |

Keyframe, dependency-frame, decoded-frame, GOP, and access-amplification
controls had zero mismatches. Eleven of 48 individual seek medians regressed
within timing noise (worst −3.74%), while aggregate useful throughput cleared
the 5% timing tolerance.

All 54 release tests passed. Low-level controls cover sparse/dense/extreme
alphabets, truncated and trailing payloads, state validation, and canonical
final states. Separate q90 and q100 controls proved that the new encoder's
decoded samples exactly match the scalar encoder, and that the new decoder
still decodes preserved scalar version-3 streams.

Two benchmark-tooling faults were found and corrected: mixed-version rows
were originally parsed positionally despite newer tile-geometry columns, and
`cargo test` did not refresh the standalone release benchmark binary. Named
column extraction, equal-column validation, and an explicit binary rebuild
now guard both cases.

## Artifacts

- final focused A/B: `artifacts/exp0068-budgeted-focused.tsv`
  (`ebab3b35a4ca8cec5767d44f111beb0b8202312210b28a2829d66783580fc831`);
- final standard matrix: `artifacts/exp0068-budgeted-corpus.tsv`
  (`83c2b7b4752356c26bda8eaaa6fd1a42595c68d92acd420368eea50be1703c33`);
- final access matrix: `artifacts/exp0068-budgeted-access.tsv`
  (`61cf6486278e7c1a608085c3eccc9a2d87e44d249d53125151f30719b9858441`);
- ungated focused A/B: `artifacts/exp0068-four-state-focused.tsv`
  (`3d94376578f5b544afa1daf7d02d3e906d8833967198f9dd0e17e1cfb5f7d51f`);
- ungated standard matrix: `artifacts/exp0068-four-state-corpus.tsv`
  (`ec457959097476743d8e98dc6e93fe51ac88f864fa2a41dd758f6ebec8d3a6df`);
- failed-kernel profile: `artifacts/exp0068-four-state-perf.data`
  (`8a21d4dd1c6f0126dacfc5dc062e0915d7dbf7625b28630b84bd2a354398b324`);
- exact source delta from `95d4fe0`:
  `artifacts/exp0068-four-state.patch`
  (`e15d191e40c9779f57e7a864987cff544c4752ffb15bacca2ea756fa4c030b89`);
- measured candidate binary:
  (`ca735038eba7b24123e68804d496b7d4cb460bae19f962418b22177f9fd6900b`);
- promotable release rebuild after adding the test-only control:
  `target/release/fastvid`
  (`d4d7edaf68a67601f753652757d62bcc49ff237e9ef0954ad0174ddc45322a14`).

## Decision

**Accepted.** The final candidate is within the frontier's 1% byte tolerance
and materially improves decode and access beyond the 5% timing tolerance,
while preserving quality and dependency depth. Promote it over EXP-0055 in
the maximum-compression slot. Explicit SIMD remains a separate future
experiment: this result shows that state independence and batched control
flow should be established before intrinsics are considered.
