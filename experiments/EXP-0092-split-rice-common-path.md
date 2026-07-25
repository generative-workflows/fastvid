# EXP-0092 — Split Rice common path

Status: **REJECTED**

## Classification

**Speed exploitation** — isolate the code-layout opportunity identified by
EXP-0090 without repeating EXP-0080's bulk-flush change.

## Hypothesis

The current out-of-line Rice writer accounts for 26.51% of encode-only
samples and saves six registers on every symbol. Inlining only its bounded
short-code path while moving the rare overflow path to a cold, non-inlined
helper should improve matched q90 one-thread encode by at least 3%.

EXP-0080 variant A inlined the whole function and regressed through caller
bloat. Its variant B also replaced byte pushes with a slice append, which
independently regressed short codes. This experiment preserves the current
common-path instructions and changes only their placement.

## Modification

1. Force-inline the quotient, fit test, word assembly, and existing
   `flush_bytes` call.
2. Pass the already computed quotient to a cold, never-inlined overflow
   helper.
3. Preserve `Vec::push` flushing, capacity, syntax, parameter selection,
   prediction, reconstruction, and decoding exactly.

## Gate

- byte- and metric-identical focused q90 streams;
- at least 3% matched q90 one-thread encode improvement;
- decode no worse than 5%;
- the release binary has a separate cold overflow symbol and no standalone
  common-path writer symbol;
- strict Clippy, formatting, and Rice boundary tests pass; and
- no slow-tier run unless the focused gate passes.

## Result

Strict release Clippy, formatting, and the exhaustive Rice
boundary/fallback test passed. The release binary contained only the
separate `put_rice_overflow` symbol, confirming that the common path was
inlined as designed.

The balanced two-trial screen initially showed +1.705% encode on the
matched 10-bit sample and +4.268% on 16-bit. Because that was directionally
interesting but near the gate, a six-trial focused run was used to resolve
the noise:

| Depth | Baseline encode | Candidate encode | Delta | Decode delta | Bytes |
|---:|---:|---:|---:|---:|---:|
| 10-bit | 71.118 MP/s | 70.122 MP/s | -1.401% | +0.582% | identical |
| 16-bit | 67.585 MP/s | 69.066 MP/s | +2.191% | -2.118% | identical |
| geometric aggregate | 69.329 MP/s | 69.592 MP/s | +0.379% | -0.777% | identical |

PSNR and block SSIM were identical. The current matched 10-bit path
regressed and the aggregate result missed the 3% gate by a wide margin.
No slow-tier or OpenAPV run was performed.

Artifacts:

- six-trial focused results:
  `artifacts/exp0092-rice-split-focused.tsv`
  (`87abde9293fd15904b45e9de348ca374108a5715368baa15c534dc34e87f0cbc`);
- initial two-trial screen:
  `artifacts/exp0092-rice-split-smoke.tsv`
  (`595e10948ce68a26758e121f3bcfdf6e88f331f18ebb6015ec2c7491bca4a688`);
- candidate binary:
  `artifacts/frontier/fastvid-speed-exp0092-rice-split`
  (`d063649d153b4391e2f7418414cb0a090b7c12d743eea96757eba3fc37d90f8a`).

## Decision

Reject the split and restore EXP-0088 exactly. Together with EXP-0080,
this shows that Rice code placement is workload-sensitive and does not
offer a general matched-path speed win. The next speed experiment should
target the fused prediction/reconstruction loop identified by EXP-0090,
not another local writer-layout variant.

## References

- [Research 0019](../research/0019-modern-integer-entropy-kernels.md)
- [EXP-0080](EXP-0080-inlined-rice-writer.md)
- [EXP-0090](EXP-0090-post-pack-speed-profile.md)
