# EXP-0080 — Inlined high-bit Rice writer

Status: **REJECTED**

## Classification

**Speed exploitation** — remove per-residual call and byte-flush overhead
identified by EXP-0079 from the isolated unified speed branch.

## Hypothesis

LLVM leaves `BitWriter::put_rice` out of line in the EXP-0078 release binary.
Forcing its common path inline will eliminate a six-register
prologue/epilogue and expose constant propagation around the caller's
tile-local Rice parameter. This should improve matched q90 one-thread encode
throughput by at least 5% without changing one byte of output.

If forced inlining is below that gate, replacing the byte-at-a-time common
flush with one bounded slice append may reduce repeated `Vec::push` capacity
checks. That variant remains a separate sub-result and must also be
byte-identical.

## Modification

Starting from the preserved EXP-0078 source:

1. Variant A adds a forced-inline attribute to `BitWriter::put_rice` and
   changes no logic.
2. Variant B, only if needed, emits the common path's complete bytes with one
   safe slice append while retaining the existing long-code fallback.
3. Preserve syntax, predictor, Rice parameter selection, allocation capacity,
   decoder, and quality mapping.

No unsafe code or target-specific intrinsic is introduced.

## Fast test

On the checksummed 1280x720x24 native-10-bit matched sequence at q90,
one thread, GOP 1:

- build each candidate in release mode;
- verify encoded bytes and reconstruction metrics exactly match EXP-0078;
- warm each binary and run at least six balanced/rotated trials;
- compare median codec-reported encode/decode throughput; and
- inspect the release symbol table or a short profile to confirm that the
  writer call was actually removed.

## Gate

- at least 5% one-thread encode improvement;
- byte-identical q90 stream and identical quality metrics;
- decode no worse than 5%;
- release tests, strict Clippy, and formatting pass for the candidate's
  declared policy; and
- no larger corpus or OpenAPV confirmation unless the fast gate passes.

## Result

Variant A removed the release symbol exactly as intended, but forcing the
entire writer inline regressed every encode cell:

| Quality | Threads | Baseline encode | Inline encode | Change |
|---:|---:|---:|---:|---:|
| 90 | 1 | 67.827 MP/s | 58.679 MP/s | -13.49% |
| 90 | 4 | 195.042 MP/s | 164.384 MP/s | -15.72% |
| 100 | 1 | 64.497 MP/s | 56.739 MP/s | -12.03% |
| 100 | 4 | 171.303 MP/s | 158.025 MP/s | -7.75% |

Every stream was byte-identical to EXP-0078; q90 retained 52.001930 dB
Y-PSNR, 0.99373056 SSIM, and maximum error 4, while q100 remained exact.
One-thread q90 decode changed from 65.518 to 65.785 MP/s (+0.41%).
Eliminating the call was therefore real, but inlining the full rare-code
fallback bloated the caller enough to overwhelm the saved prologue.

Variant B split the rare long-code path into a cold, non-inlined method and
used one `extend_from_slice` for the common path's complete bytes. It reduced
but did not reverse the regression:

| Quality | Threads | Baseline encode | Bulk-flush encode | Change |
|---:|---:|---:|---:|---:|
| 90 | 1 | 67.584 MP/s | 63.675 MP/s | -5.78% |
| 90 | 4 | 179.084 MP/s | 173.879 MP/s | -2.91% |
| 100 | 1 | 62.675 MP/s | 59.347 MP/s | -5.31% |
| 100 | 4 | 173.069 MP/s | 164.699 MP/s | -4.84% |

Variant B was also byte- and metric-identical. One-thread q90 decode changed
from 65.058 to 65.149 MP/s (+0.14%). The bounded slice append pays more setup
than byte-at-a-time pushes for the short Rice codes in this workload.

Because neither candidate passed the fast performance gate, full corpus,
OpenAPV, test-suite, and Clippy confirmation were intentionally skipped.
Both release builds passed formatting before measurement. Production source
was restored exactly after preserving the rejected binaries.

Artifacts:

- Variant A focused matrix:
  `artifacts/exp0080-inline-focused.tsv`
  (`08f1ec071ae541c365f94f712c3d1cd6b437662998fc7f48e3396f682d38c8f0`);
- Variant A binary:
  `artifacts/frontier/fastvid-speed-exp0080-inline`
  (`e934be9d81cbb0f5ab9934536e50c3891973c36e85630084b48684cd03fb2f20`);
- Variant B focused matrix:
  `artifacts/exp0080-bulk-focused.tsv`
  (`312bf6e855e33e8249be503fe154730775d85a228036e4bc53b88b69b0c79e58`);
- Variant B binary:
  `artifacts/frontier/fastvid-speed-exp0080-bulk`
  (`71f1c0475061601f5b430e938189be0862c77d3fc6f53c5e85ee7c808ca24397`).

## Decision

Reject both variants and retain EXP-0078 unchanged. A hot symbol is not by
itself evidence that forced inlining or wider library operations will help:
the existing out-of-line writer keeps the causal predictor loop compact, and
the preallocated vector makes its capacity branch inexpensive.

Do not continue local `BitWriter` reshaping without a different algorithmic
design, such as batching multiple Rice symbols into a word. Move the next
independent exploitation experiment to the equally large
prediction/reconstruction kernel identified by EXP-0079. Keep SIMD and
causal-dependency changes attributable rather than combining them with this
rejected writer path.

## References

- [Research 0019](../research/0019-modern-integer-entropy-kernels.md)
- [EXP-0077](EXP-0077-high-bit-prefix-rice-streaming.md)
- [EXP-0078](EXP-0078-unified-speed-frontier.md)
- [EXP-0079](EXP-0079-unified-speed-profile.md)
