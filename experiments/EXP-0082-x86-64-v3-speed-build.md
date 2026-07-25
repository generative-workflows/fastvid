# EXP-0082 — x86-64-v3 speed build

Status: **REJECTED**

## Classification

**Speed exploitation / deployment experiment** — determine whether LLVM can
use the same AVX2-class ISA available to the matched OpenAPV build without
unsafe code or handwritten intrinsics.

## Hypothesis

The preserved EXP-0078 Rust binary is built for generic x86-64, while pinned
OpenAPV compiles transform/quantization kernels with `-mavx2`. Rebuilding the
unchanged Fastvid speed source for the standardized x86-64-v3 microarchitecture
level may improve matched q90 one-thread encoding by at least 5% through
AVX2, BMI2, and improved instruction selection, with byte-identical output.

## Modification

Apply the reproducible EXP-0078 speed patch to its current source base and
compile the unchanged code with:

```text
-C target-cpu=x86-64-v3
```

Use a separate Cargo target directory and preserve the resulting binary. Do
not change source, syntax, quality, predictor, entropy selection, or decoder.
x86-64-v3 is tested before `target-cpu=native` because it is a standardized
AVX2 deployment tier and is closer to OpenAPV's measured ISA boundary.
If v3 fails the speed gate, a host-native build may be measured as a
diagnostic only. It distinguishes missing microarchitecture scheduling from
structural loop dependence, but cannot enter the portable frontier without a
runtime-dispatched implementation.

## Fast test

Use the focused six-trial balanced high-bit harness at q90/q100 and one/four
threads:

- compare against the exact EXP-0078 generic binary;
- require byte- and metric-identical streams;
- report encode/decode changes;
- inspect compiler output or binary instructions to confirm the requested
  target was honored; and
- run the full candidate correctness controls only if the speed gate passes.

If the v3 result is flat, run the same focused matrix once with
`target-cpu=native`; do not promote that host-specific result.

## Gate

- at least 5% matched q90 one-thread encode improvement;
- bytes and metrics exactly equal EXP-0078;
- q90 decode no worse than 5%;
- no source change beyond the already preserved EXP-0078 patch; and
- label the result as an x86-64-v3 deployment tier, never as a portable
  generic-binary improvement.

If the gate passes, confirm against OpenAPV and the complete native
supplement before changing the frontier.

## Result

The x86-64-v3 binary contained AVX/AVX2 instructions, including YMM
`vpbroadcastq` and `vpaddq`, and produced byte- and metric-identical streams.
Its focused medians were:

| Quality | Threads | Generic encode | v3 encode | Change |
|---:|---:|---:|---:|---:|
| 90 | 1 | 67.867 MP/s | 67.845 MP/s | -0.03% |
| 90 | 4 | 187.474 MP/s | 185.483 MP/s | -1.06% |
| 100 | 1 | 64.273 MP/s | 65.038 MP/s | +1.19% |
| 100 | 4 | 169.415 MP/s | 179.738 MP/s | +6.09% |

One-thread q90 decode changed from 65.251 to 64.451 MP/s (-1.23%).
The primary encode gate therefore failed: standardized AVX2-class code
generation does not improve the causal q90 loop.

The predeclared host-native diagnostic emitted AVX-512 ZMM instructions and
measured:

| Quality | Threads | Generic encode | Native encode | Change |
|---:|---:|---:|---:|---:|
| 90 | 1 | 67.767 MP/s | 70.307 MP/s | +3.75% |
| 90 | 4 | 172.863 MP/s | 192.260 MP/s | +11.22% |
| 100 | 1 | 64.184 MP/s | 66.796 MP/s | +4.07% |
| 100 | 4 | 171.966 MP/s | 185.935 MP/s | +8.12% |

Native q90 one-thread decode improved 0.66%. This is useful diagnostic
evidence but remains below the 5% gate and is not a portable,
runtime-dispatched implementation. Even the host-specific 70.307 MP/s result
is 12.59% below OpenAPV `fastest` at 80.431 MP/s (OpenAPV is 14.40% faster).

All streams retained 18,882,860 bytes at q90, 52.001930 dB Y-PSNR,
0.99373056 SSIM, and maximum error 4. q100 retained 32,246,235 bytes and exact
reconstruction. No source changed, so full source-level correctness
confirmation was not triggered; the generic production tree was restored
exactly.

Artifacts:

- x86-64-v3 focused matrix:
  `artifacts/exp0082-v3-focused.tsv`
  (`c19ce4aa5c36c02f16936dea04e5b4029a8d6c9a08510d73b49b0d2a5cd62a97`);
- x86-64-v3 binary:
  `artifacts/frontier/fastvid-speed-exp0082-v3`
  (`241e06313ced9691255654b0cd05953c84ddac57b2256f38ca748a4f5c99a410`);
- host-native focused matrix:
  `artifacts/exp0082-native-focused.tsv`
  (`d4a0944b5f43f03684e018faf93bf08e1a08b271025c935ea3c969d990b0fd25`);
- host-native binary:
  `artifacts/frontier/fastvid-speed-exp0082-native`
  (`7ed9de56d892741d8bfb579c166f42aa0e8ca4d5bc184d13762e5689c6d648ec`).

## Decision

Reject x86-64-v3 as a frontier replacement and retain generic EXP-0078.
Target-wide AVX2 is effectively neutral at the matched q90 boundary, proving
that merely enabling SIMD does not remove the predictor/writer dependencies
seen in EXP-0079.

Retain the host-native binary only as a diagnostic upper bound. A future
runtime-dispatched kernel could combine microarchitecture-specific scheduling
with an algorithmic change, but global `target-cpu=native` is not a
deployable codec strategy. Do not claim the four-thread gains as resolution
of the one-thread OpenAPV target.

## References

- [Research 0012](../research/0012-simd-cache-profiling.md)
- [EXP-0078](EXP-0078-unified-speed-frontier.md)
- [EXP-0079](EXP-0079-unified-speed-profile.md)
- [EXP-0080](EXP-0080-inlined-rice-writer.md)
