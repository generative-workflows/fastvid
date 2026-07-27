# EXP-0139 — CUDA encoding feedback baseline

Status: **ACCEPTED**

## Hypothesis

A mixed-resolution, real/lossless/rendered corpus panel measured before CUDA
encoder optimization will expose target failures and scaling behavior hidden
by a single real-world 4K decode result.

## Modification

Add `scripts/benchmark-cuda-feedback.sh` with two scopes:

- `quick`: eight pinned 1080p/2K/4K first frames for routine feedback;
- `full`: the first frame of all 24 corpus-v3 codec samples.

For q90 and q100 it preserves three post-warm-up Rust encode trials at one and
four threads, complete rate/PSNR/block-SSIM/error/XPSNR controls, CUDA decode
from DRAM and VRAM, environment data, corpus/binary hashes, and per-sample
rows. `scripts/summarize-cuda-feedback.py` reports aggregate and minimum GP/s,
per-target pass counts, and an explicit 1080p slice. No codec optimization or
format change is part of this experiment.

## Test

Run on the NVIDIA L40 with the release Rust v5 oracle and current PyTorch CUDA
extension:

```sh
scripts/benchmark-cuda-feedback.sh \
  target/release/fastvid artifacts/corpus-v3 \
  artifacts/exp0139-cuda-feedback 3 full
scripts/summarize-cuda-feedback.py \
  artifacts/exp0139-cuda-feedback \
  benchmarks/v5-cuda-feedback.md \
  benchmarks/v5-cuda-feedback-summary.tsv
```

The release binary SHA-256 was
`224782496805cc15ee86290515010804b613ea4375a96a986accd86a7e654a69`;
the extension SHA-256 was
`d48c001c6d38f18561b4517d9c47d3fb19e8b8e543255cd6c6e5f5de0988be3a`.

## Result

At q90, all 24 samples exceeded 50 dB luma XPSNR (minimum 51.9589 dB),
but total compression was 11.687517x and only 8/24 individual samples exceeded
15x. Rust encode geometric mean was 0.033390 GP/s at one thread and 0.086448
GP/s at four threads. Complete-call CUDA decode geometric mean was 3.029134
GP/s from DRAM and 2.535451 GP/s from VRAM; only 5/24 and 4/24 samples,
respectively, exceeded 5 GP/s.

The 15-sample 1920x1080 q90 slice was more demanding than the original 4K
headline: CUDA decode was 2.921790 GP/s from DRAM and 2.372671 GP/s from VRAM,
while four-thread Rust encode was 0.084454 GP/s. Its total compression was
8.366150x, with 3/15 samples above 15x. Q100 reconstructed exactly on all 24
samples.

Raw artifact SHA-256 values:

- encode: `1ea5e2ad456d14721044612db17533037d65367ffbb39c8e9bc9d8b517c3d188`;
- quality: `6bc275ae0f970e0f392b5423580ea4d8363199c7a5ec4e5002545fa1d01b0d74`;
- decode: `e16bb55e57ba467bdd95f428ed5849aa4c3b31924dbbe4c7b3f509bcd49e7f22`;
- environment: `8a3c2ade75ff76b10e39d2654bfe527b124ace1fc235103b3912b44b4cda3e85`.

## Decision

Accept this as the pre-optimization feedback baseline. It disproves any
corpus-wide inference from the earlier single 4K decode result and makes
small-frame launch/orchestration overhead, encode throughput, and compression
the measured gaps. CUDA encoder candidates must reproduce Rust bytes before
being compared, then improve against these rows without losing q100 exactness
or the q90 quality floor.

The panel samples only one frame from each corpus item and is therefore a
spatial confirmation tier. Whole 24-frame sequences, selected-frame APIs, and
temporal behavior remain separate required confirmation work.

## References

- [Evaluation methodology](../EVALUATION_METHODOLOGY.md)
- [Research 0042](../research/0042-gpu-variable-output-assembly.md)
- [Research 0043](../research/0043-xpsnr-quality-metric.md)
- [Research 0044](../research/0044-open-high-resolution-corpus.md)
- [EXP-0135](EXP-0135-cpu-gpu-baseline.md)
- [EXP-0136](EXP-0136-corpus-v3-native-2k-4k.md)
- [EXP-0137](EXP-0137-cuda-v5-decoder-baseline.md)
