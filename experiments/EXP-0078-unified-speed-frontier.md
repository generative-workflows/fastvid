# EXP-0078 — Unified 8/high-bit speed frontier

Status: **ACCEPTED**

## Classification

**Frontier integration exploitation** — combine the accepted 8-bit
fixed-gradient speed policy from EXP-0060 with the accepted high-bit fixed
gradient/prefix-Rice policy from EXP-0077 in one reproducible binary.

## Hypothesis

A single speed binary can retain the current internal 8-bit speed point within
5% while reproducing EXP-0077's high-bit bytes, metrics, and throughput. This
will make the three-slot frontier semantically honest across supported bit
depths without adding a fourth active version.

## Modification

Starting from current maximum-compression source:

- 8-bit intra tiles directly use clamp-gradient prediction;
- 8-bit preferred inter tiles directly use temporal prediction;
- the 8-bit speed path uses only zero-run/Rice entropy, avoiding
  maximum-tier rANS planning;
- high-bit tiles use EXP-0077's fixed-gradient middle-row Rice estimator,
  direct Rice emission, and exact zero-run fallback;
- all decoder modes remain supported;
- preserve the result as an isolated frontier binary and source patch rather
  than replacing production maximum-compression source.

## Test

1. Run correctness controls for q100, q90 error bounds, independent tile/mode
   decode, malformed streams, and high-motion fallback.
2. Run the automatic three-version 8-bit fast frontier with six rotated
   trials, substituting only the speed binary.
3. Require the high-bit q90 matched stream and metrics to equal EXP-0077 and
   rerun the focused six-trial comparison.
4. If both pass, update `frontier.json`, `FRONTIER.md`, the automatic graph,
   and the matched OpenAPV panel.

## Gate

- internal 8-bit speed encode/decode geomeans no worse than 5% below EXP-0060;
- internal speed compression no worse than 1%;
- matched high-bit q90 bytes and quality exactly equal EXP-0077;
- matched high-bit encode/decode remain within 5% of EXP-0077 medians;
- at most three active frontier slots;
- hashes and source patch reproduce the unified binary.

## Result

The direct six-trial 8-bit A/B against the preserved EXP-0060 speed binary
produced byte-identical streams in all four cases:

| Case | Encode change | Decode change |
|---|---:|---:|
| camera 1080p | -2.80% | -1.38% |
| cuts temporal 1080p | +3.69% | +1.05% |
| grid 4K | +0.26% | +1.47% |
| UI temporal 720p | +14.64% | +5.94% |
| **Geometric mean** | **+3.74%** | **+1.74%** |

The first A/B output was invalid because the older binary omitted newer
diagnostic columns and `benchmark-ab-feedback.sh` copied its header onto wider
candidate rows. The harness was corrected to extract every binary's fields by
name and require all normalized columns; the invalid artifact was overwritten
before analysis. This applies the cross-version schema rule already present
in the evaluation methodology.

The fresh rotated three-version internal matrix retained exactly three slots:

| Slot | Compression | Encode | Decode | Playback bitrate |
|---|---:|---:|---:|---:|
| speed | 13.353556x | 107.606780 MP/s | 143.941592 MP/s | 68.853922 Mb/s |
| practical compression | 24.547776x | 29.150468 MP/s | 133.206242 MP/s | 37.455311 Mb/s |
| maximum compression | 33.588694x | 24.477185 MP/s | 99.308986 MP/s | 27.373634 Mb/s |

The unified speed binary measured 9.12% below the historical speed encode
snapshot, but that cross-run comparison is contradicted by the balanced
same-run +3.74% A/B and by 1--2% movement in both unchanged comparison slots.
Promotion therefore uses the direct A/B for dominance and the fresh matrix
only for current graph coordinates.

The high-bit focused A/B against the exact EXP-0077 binary was byte-identical
at q90/q100. One-thread q90 was 67.017 versus 67.168 MP/s (-0.22%) and q100
was 64.758 versus 65.041 MP/s (-0.44%). Four-thread differences were +7.10%
at q90 and -3.95% at q100. All are inside the declared no-worse-than-5% gate
for the required one-thread comparison, and reconstruction metrics were
identical.

The refreshed matched OpenAPV panel measured:

| Codec | Control | Ratio | Encode | Decode | Y PSNR |
|---|---:|---:|---:|---:|---:|
| Fastvid speed | q90 | 4.685392x | 67.301 MP/s | 64.511 MP/s | 52.001930 dB |
| Fastvid practical | q90 | 5.307903x | 16.642 MP/s | 59.517 MP/s | 52.002293 dB |
| Fastvid maximum | q90 | 5.307903x | 16.802 MP/s | 59.620 MP/s | 52.002293 dB |
| OpenAPV `medium` | QP22 | 4.408004x | 17.666 MP/s | 62.658 MP/s | 51.534665 dB |
| OpenAPV `fastest` | QP23 | 4.464067x | 80.431 MP/s | 61.956 MP/s | 51.735588 dB |

At q90, Fastvid speed uses 4.72% less bitrate at 0.266 dB higher Y-PSNR and
decodes 4.12% faster; OpenAPV encodes 19.51% faster. At the distinct
high-fidelity boundary, Fastvid speed q100 is exact at 2.743688x, 64.077
MP/s, and 257.969880 Mb/s. OpenAPV `fastest` QP0 has maximum error 2 at
1.965097x, 62.481 MP/s, and 360.180000 Mb/s. Fastvid is 2.55% faster to
encode and uses 28.38% less bitrate at that higher-fidelity boundary, but the
rows are not called nominal-control matches.

The unified branch passed 49/54 library tests. Five maximum-policy assertions
failed because the isolated speed encoder deliberately chooses fixed
gradient instead of the predictor oracle/legacy Paeth and deliberately omits
8-bit rANS selection. Generic q100 exactness, q90 bounds, malformed streams,
all decoder modes, independent tile decode, Rice/rANS payload decoding,
metrics, and model tests passed. Strict Clippy and formatting passed. The
restored maximum-compression working source subsequently passed all 54
library tests and both squeeze-model tests.

Artifacts:

- unified source patch:
  `artifacts/frontier/exp0078-unified-speed.patch`
  (`ceb74c26f89f1d32804f9c6671152771da520d57e0b20440481dbb0d9dae53df`);
- unified release binary:
  `artifacts/frontier/fastvid-speed-exp0078`
  (`bf1002e7e790bb5607180ff2874edd57957536c83cce620982f0a6999614ccb3`);
- normalized direct 8-bit A/B:
  `artifacts/exp0078-unified-speed-ab.tsv`
  (`c43d4a5f82784161ab5e8af568d2ee1bbda120a56a7380f808e287bf0e4a9dd1`);
- internal frontier matrix:
  `artifacts/exp0078-unified-frontier.tsv`
  (`f54abfd9576ca2c6da56001088d060e12387688c95345001d728f0fb56845546`);
- direct high-bit A/B:
  `artifacts/exp0078-unified-highbit-ab.tsv`
  (`eac899d6324d4b71a3da48199c437aa7d4ae7bc434bc64d82dc08e55445c664a`);
- matched external matrix:
  `artifacts/exp0078-openapv-frontier.tsv`
  (`56c3cbe8a66c37a1c2e96ca616fa4f4df22e7c9ce6ea87a50582a505e3f5916e`);
- internal graph/summary:
  `0cd4cc6fed2ec6ac6e1013c3d436c1d050ffe6cff23110094e33e30ae9aede35`,
  `adcb19d37603a211711c877a843ee1f520b7fb06592652a4de987ff20b32d2c1`;
- external graph/summary:
  `697a31f9a28d7381ded943b439a9d8771705308f4101ea273f2ecf1855d4e3c2`,
  `eeae6bfec919da716cb0ee5c37782a9e3b70886866aba66c707877af0d063f23`;
- normalized A/B harness:
  `7f384e8a21210835b27272c3e43f1bd35fc6111276ca39431d81823315ab60e7`.

## Decision

Accept and replace the EXP-0060 speed slot with the unified EXP-0078 binary.
Retain exactly three active frontier versions. The old speed binary remains a
historical artifact. The updated external panel now exposes both the q90
19.51% encode deficit and the exact high-fidelity point where Fastvid is
already 2.55% faster than OpenAPV `fastest`.

Restore maximum-compression working source after preserving the patch and
binary. Continue optimization against the q90 deficit; do not weaken the
quality target to claim completion from the distinct q100 boundary.

## References

- [EXP-0060](EXP-0060-fixed-gradient-speed-tier.md)
- [EXP-0061](EXP-0061-three-version-frontier.md)
- [EXP-0073](EXP-0073-matched-openapv-frontier.md)
- [EXP-0077](EXP-0077-high-bit-prefix-rice-streaming.md)
