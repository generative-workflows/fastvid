# EXP-0145 — CUDA warp-parallel fixed-block emission

Status: **ACCEPTED**

## Hypothesis

Camera-pontegana q90 spends about 705 us in emission and compresses only 4.44x.
EXP-0144 proved that adding Rice parallelism does not change that time. The
remaining fixed-block path assigns one thread to each 128-symbol block, makes
that thread recompute all prior block widths, and emits every bit serially.
Assigning one warp per block will reduce camera-pontegana emission by at least
30% without slowing non-block controls by more than 5%.

## Modification

Build a compact host-side list of shards selected for fixed-block packing from
the analysis data already transferred for stream assembly. Keep zero-run and
Rice emission in the existing kernel. Launch a separate block-pack kernel only
for selected shards, with one warp per 128-symbol block; compute widths and
byte offsets cooperatively and use aligned atomic OR for exact disjoint stream
bits.

## Test

Require whole-stream CUDA/Rust byte identity across the conformance suite.
Compare q90 complete-call and stage times on camera-pontegana (slow 1080p),
camera-cholla (fast 1080p), and Calotes (real-world 4K). Accept only if the slow
sample's emission falls by at least 30% and neither control regresses by more
than 5% in complete-call throughput.

## Result

Whole-stream conformance passed for 10-, 12-, and 16-bit inputs, q90 and q100,
odd edge tiles, and all entropy modes; CUDA output remained byte-for-byte
identical to Rust. On camera-pontegana, total emission fell from 705.375 us to
280.736 us (60.2%): 275.680 us in the existing non-block kernel plus 5.056 us
in the new block kernel. Complete-call throughput improved from 1.536020 to
2.114891 GP/s (+37.7%). Camera-cholla changed from 3.264633 to 3.232495 GP/s
(-1.0%), while Calotes 4K improved from 3.704638 to 3.723455 GP/s (+0.5%).

## Decision

Accept. The emission reduction exceeds the 30% gate, both controls remain
inside the 5% regression bound, and all bytes remain exact.

The refreshed 24-sample first-frame panel measured 2.152290 GP/s q90 and
2.135719 GP/s q100 geometrically. On the 15-sample 1080p q90 slice, geometric
throughput was 2.309584 GP/s and the minimum was 1.908984 GP/s, up from
2.279231 and 1.536020 GP/s in the EXP-0143 panel. Q90 retained 24/24 samples
above 50 dB XPSNR and 8/24 above 15x compression. All 48 CUDA streams matched
Rust bytes.

Reproducibility hashes:

- CUDA extension: `eb19e22cd03ebfb8a7a2d8a8e3d8afc13cc28b0255c158aa88921a754b73d1de`
- Rust binary: `224782496805cc15ee86290515010804b613ea4375a96a986accd86a7e654a69`
- CUDA encode rows: `ed97a70dc8ee2f8c01cf23599c87132bc9917ca0b8ec2ad7b883a4991ef91940`
- CUDA decode rows: `3106dce31d32e449b66f1f712cc0af42998c40cd8f0e7daa5c2776b6697bb87b`
- quality rows: `6bc275ae0f970e0f392b5423580ea4d8363199c7a5ec4e5002545fa1d01b0d74`

## References

- [EXP-0144](EXP-0144-cuda-multiwarp-rice-emission.md)
- [CUDA feedback summary](../benchmarks/v5-cuda-feedback-encoder.md)
