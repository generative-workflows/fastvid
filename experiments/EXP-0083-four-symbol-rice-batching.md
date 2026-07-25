# EXP-0083 — Four-symbol Rice batching

Status: **REJECTED**

## Classification

**Speed exploitation** — replace per-residual Rice writer calls with exact
four-symbol packing in the isolated EXP-0078 high-bit speed path.

## Hypothesis

EXP-0079 attributes 17.25% of the whole matched command to the out-of-line
per-symbol Rice writer. EXP-0080 showed that inlining the large writer is
counterproductive. Packing four short Rice codes into one temporary word can
reduce calls, writer-state traffic, and capacity checks by approximately 4x
while preserving the exact bit sequence. It should improve q90 one-thread
encode throughput by at least 5%.

## Modification

Starting from the preserved EXP-0078 source:

1. process fixed-gradient source rows in exact groups of four samples while
   retaining causal reconstructed neighbors;
2. collect the four folded residuals in a fixed stack array;
3. when their combined Rice code is at most 64 bits, concatenate the exact
   LSB-first codes and append them through one writer method;
4. fall back to the existing per-symbol writer for long groups and row
   remainders; and
5. leave parameter selection, entropy mode, predictor, syntax, decoder,
   allocation capacity, and quality mapping unchanged.

The candidate uses safe Rust and no target-specific intrinsics. A unit test
must compare batched and scalar writers over boundary values and parameters.

## Fast test

First run the new writer equivalence unit test and the focused six-trial
q90/q100 high-bit A/B against EXP-0078. Require byte-identical streams and
metrics. If the gate passes, profile the writer symbol and then confirm on
the complete native supplement.

## Gate

- at least 5% matched q90 one-thread encode improvement;
- byte-identical q90/q100 streams and identical metrics;
- q90 decode no worse than 5%;
- writer equivalence across short, crossing, and fallback code groups;
- strict Clippy, formatting, and relevant correctness tests pass; and
- no full OpenAPV matrix unless the focused gate passes.

## Result

The batched writer passed its scalar-equivalence test for parameters 0--16,
bit alignments 0--7, short groups, 64-bit crossings, and long-code fallback.
The focused A/B remained byte- and metric-identical:

| Quality | Threads | Scalar encode | Rice-4 encode | Change |
|---:|---:|---:|---:|---:|
| 90 | 1 | 68.525 MP/s | 70.688 MP/s | +3.16% |
| 90 | 4 | 190.717 MP/s | 185.777 MP/s | -2.59% |
| 100 | 1 | 62.336 MP/s | 66.490 MP/s | +6.66% |
| 100 | 4 | 179.486 MP/s | 197.994 MP/s | +10.31% |

q90 retained 18,882,860 bytes, 52.001930 dB Y-PSNR, 0.99373056
SSIM, and maximum error 4. q100 retained 32,246,235 bytes and exact
reconstruction. One-thread q90 decode changed from 66.365 to 64.212 MP/s
(-3.24%), inside its tolerance; four-thread decode fell 5.65%.

The fixed group reduces scalar writer overhead and is more effective at q100,
but q90 one-thread misses the 5% advancement gate and q90 parallel behavior
does not support promotion. Strict Clippy, formatting, and the focused writer
test passed. The full corpus and OpenAPV matrix were intentionally skipped.
Production source was restored exactly after preserving the candidate.

Artifacts:

- focused matrix:
  `artifacts/exp0083-rice4-focused.tsv`
  (`4ba958a23a335ba8431a346df7cfd3e039deb33f02baf28225e64662f7d10e3c`);
- release binary:
  `artifacts/frontier/fastvid-speed-exp0083-rice4`
  (`c6acf9f3789a0d1a578dfd15227353191d182b1a61f87928a72486482ca692cb`);
- exact source patch:
  `artifacts/frontier/exp0083-rice4.patch`
  (`ce14fa910c4248fbd520657ca7ab92bd3986a8306a789fe366db652534e1233d`).

## Decision

Reject as a standalone frontier replacement. Four-symbol packing demonstrates
that call/state batching is a valid direction, but the generic implementation
does not save enough q90 time and slightly harms the parallel cells.

Retain the result as a stepping stone. A follow-up may specialize the two
observed q90 Rice parameters outside the pixel loop so LLVM can constant-fold
quotient/remainder operations, or compare a larger group. It must beat
EXP-0078 directly and may not combine with the host-native EXP-0082 build
until a portable source improvement independently passes.

## References

- [Research 0019](../research/0019-modern-integer-entropy-kernels.md)
- [EXP-0077](EXP-0077-high-bit-prefix-rice-streaming.md)
- [EXP-0079](EXP-0079-unified-speed-profile.md)
- [EXP-0080](EXP-0080-inlined-rice-writer.md)
- [EXP-0082](EXP-0082-x86-64-v3-speed-build.md)
