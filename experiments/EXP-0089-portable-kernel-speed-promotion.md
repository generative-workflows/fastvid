# EXP-0089 — Portable-kernel speed promotion

Status: **ACCEPTED**

## Classification

**Slow-tier confirmation** — determine whether accepted EXP-0088 replaces
EXP-0086/0087 in the speed frontier and quantify the remaining OpenAPV gap.

## Hypothesis

The byte-identical portable kernel's 7.50% focused encode improvement and
passing access/supplement evidence should make it dominate the current speed
slot. A fresh balanced matched run should reduce, but not yet eliminate, the
practical-q90 OpenAPV `fastest` encode deficit.

## Test

1. Validate source `29dcc43` and the release candidate hash in a candidate
   frontier manifest.
2. Run six balanced internal frontier trials and regenerate its summary.
3. Run six balanced matched trials for all Fastvid slots and declared
   OpenAPV controls at one/four threads.
4. Require deterministic bytes/metrics, no rate or quality change from
   EXP-0086, and no encode/decode/access regression outside 5%.
5. Regenerate graphs and promote only if the candidate remains dominant.

## Result

The internal four-case screening retained identical ratio and raised the
speed slot aggregate from the EXP-0087 run's 117.002 MP/s to 120.165 MP/s
encode. The three current rows were:

| Slot | Ratio | Encode | Decode |
|---|---:|---:|---:|
| Practical compression | 24.547776x | 29.323 MP/s | 137.890 MP/s |
| Maximum compression | 33.588694x | 24.484 MP/s | 103.488 MP/s |
| Speed | 13.353556x | 120.165 MP/s | 146.275 MP/s |

During confirmation, the external harness was corrected to treat OpenAPV as
a fixed reference. `frontier.json` now pins the complete EXP-0087 matrix and
its SHA-256. Normal runs validate that hash and the six-trial control grid,
copy the 144 OpenAPV rows, and measure only the 72 Fastvid rows. The
shortened validation proved all reused rows byte-for-byte identical. Passing
`--refresh` explicitly retains the full external rerun for a changed binary,
corpus, control set, or machine. This removes two thirds of codec
measurements from routine matched confirmation.

Fresh one-thread matched q90 results were:

| Codec | Control | Ratio | Encode | Decode | Bitrate | Y PSNR |
|---|---:|---:|---:|---:|---:|---:|
| Fastvid speed | q90 | 4.809339x | 71.290 MP/s | 68.377 MP/s | 147.169656 Mb/s | 52.001930 dB |
| Fastvid practical | q90 | 5.307903x | 16.839 MP/s | 60.454 MP/s | 133.346224 Mb/s | 52.002293 dB |
| Fastvid maximum | q90 | 5.307903x | 16.800 MP/s | 59.876 MP/s | 133.346224 Mb/s | 52.002293 dB |
| OpenAPV medium | QP22 | 4.408004x | 17.633 MP/s | 63.468 MP/s | 160.568984 Mb/s | 51.534665 dB |
| OpenAPV fastest | QP23 | 4.464067x | 81.182 MP/s | 63.471 MP/s | 158.552448 Mb/s | 51.735588 dB |

Fastvid retains 7.18% lower bitrate and 0.266 dB higher Y PSNR than OpenAPV
`fastest`. Its practical-q90 encode deficit falls from 19.35% to 12.18%;
equivalently OpenAPV is 13.88% faster. Fastvid decodes 7.73% faster. At four
threads Fastvid is 11.29% slower to encode and 21.78% faster to decode.

At the separate high-fidelity boundary Fastvid q100 is exact at 2.743688x
and 64.472 MP/s encode. OpenAPV `fastest` QP0 has maximum error two at
1.965097x and 63.200 MP/s, so Fastvid is 2.01% faster there; this does not
satisfy the practical-q90 target.

Artifacts:

- source commit: `29dcc43`;
- release binary:
  `artifacts/frontier/fastvid-speed-exp0088-word-block`
  (`adc638be500095ee9dff4e5c8030641178dd5c41517f1a7939d3e77f5a6ec8d7`);
- internal raw matrix:
  `artifacts/exp0089-frontier-fast-feedback.tsv`
  (`ebf9747df208399b6348fb279867b72448446b121c865223388819e7a30388a8`);
- internal graph/summary:
  `e5b78879db26353b5b9055b12b5c3642134a18603baee7005047f93de288016e` /
  `08f8e5b99d0812b9e61e4b9150b8ecbcea43847bdb42838790cf563ff238d1ae`;
- matched raw matrix:
  `artifacts/exp0089-openapv-frontier.tsv`
  (`81a16b95001cb3276ff790bdb59a376d3d16c3c649d3d320a4f41759d2a57dd7`);
- matched graph/summary:
  `8fe9e97f3842deaccb0c04080f8e07855f9c3b30eaa93600eb17559d2a34ff2a` /
  `ffd72cc1d3ba8c35e14d4b66c065218d2a78d50b71c5a932bcc966481b46f4a0`;
- reusable matched harness:
  `scripts/benchmark-openapv-frontier.sh`
  (`0e9b8f5ca47e63dc279f1eaa593f129906c53f24b41a68594be1a32409322076`).

## Decision

Promote EXP-0088 over EXP-0086 in the speed slot. It is byte- and
quality-identical, improves focused and matched encode materially, and passes
the broader supplement and access tolerances.

The practical-q90 OpenAPV target remains open by 12.18%. Profile source
commit `29dcc43`; do not assume the now-fast fixed-block packer remains the
next useful SIMD target. Keep the reusable OpenAPV result path as the default
slow confirmation method.

## References

- [EXP-0087](EXP-0087-block-pack-speed-promotion.md)
- [EXP-0088](EXP-0088-portable-block-pack-kernel.md)
