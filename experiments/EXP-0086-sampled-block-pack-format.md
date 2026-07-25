# EXP-0086 — Sampled block-pack format prototype

Status: **ACCEPTED**

## Classification

**Speed/format exploitation** — implement the charged EXP-0085 mode with a
content-derived selector, scalar normative coding, and no architecture
intrinsics.

## Hypothesis

A middle-row proxy can identify the q90 Cr tiles where 128-symbol fixed-width
packing beats Rice. Streaming causal residuals into one 128-value stack block
at a time should retain most of EXP-0085's 2.58% matched size saving without
the full-tile residual buffer and may reduce variable Rice-writer work enough
to improve encoding. Even a speed-neutral scalar result would establish a
sound format target for later SIMD.

## Modification

Starting from EXP-0078:

1. add one high-bit entropy mode for 128-symbol fixed-width blocks;
2. encode each block as one width byte followed by byte-aligned LSB-first
   fixed-width values;
3. reject widths outside the normative folded-residual range and nonzero
   padding/trailing bytes;
4. extend the middle-row source proxy to compare fully charged bit-pack and
   zero-run/Rice costs;
5. when selected, retain clamp-gradient causal reconstruction and buffer at
   most 128 folded values on the stack;
6. keep EXP-0078 exact fallback for every non-selected tile; and
7. add round-trip, malformed-control, padding, and independent-tile tests.

The selector must not inspect plane identity. Syntax and scalar behavior are
normative; SIMD is explicitly out of scope.

After the unrestricted sampled selector produced a 13.9% size regression on
the 16-bit motion sequence in an accidental GOP-6 stress run, audit fixed
relative confidence margins of 0%, 5%, 10%, 15%, 20%, 25%, 33%, and 50%.
Choose a margin only if it removes material false-positive cost across the
entire native high-bit supplement while retaining the aggregate matched-q90
gate; this is a content-independent uncertainty guard, not a corpus-specific
tile or plane exception.

The margin audit found no separating threshold. The first exact guard then
revealed a semantic mismatch for zero-run samples: the legacy zero-run path
re-runs the established bounded-residual encoder, whereas the fixed-gradient
pack candidate contains zigzag residuals. Therefore packing is tried only
when the sampled legacy action is fixed-parameter Rice, whose syntax the
sampled cost models exactly. An attempted full-tile Rice guard removed the
false positive too, but added roughly four percentage points of encoding
cost on the winning path and was removed after the mode-family restriction
independently restored exact 16-bit parity.

## Fast test

Compare against EXP-0078 on the focused q90/q100 matrix. Report bytes,
metrics, throughput, and selected tile count obtained from parsed streams.
If the focused result passes, run the complete native supplement and access
checks before considering specification work.

## Gate

- matched q90 bytes improve at least 1%;
- matched q90 encode is no worse than 5%;
- matched q90 decode is no worse than 10% for this scalar format prototype;
- q90 metrics exactly equal EXP-0078 and q100 remains exact;
- q100 bytes increase no more than 1%;
- at least 10% of matched q90 tiles select packing;
- malformed widths, padding, and truncation are rejected; and
- strict Clippy, formatting, and relevant tests pass.

Passing this gate advances SIMD implementation; it does not automatically
replace the frontier.

## Result

The unrestricted prototype passed the focused 10-bit gate, but an incorrectly
parameterized first supplement run usefully stressed GOP 6 at 10/12/16
threads and exposed a 13.9% 16-bit q90 size regression. The selector audit
showed why a confidence threshold was not a sound repair:

- both useful 10-bit q90 choices survived 5% but not 10% margin;
- the 1,009 false-positive 16-bit choices survived every margin through 33%
  and disappeared only at 50%; and
- the exact charged model selected no 16-bit tile.

The false positives occurred when the sampled legacy action was zero-run.
That action re-enters the established bounded-residual encoder, while the
fixed-gradient pack candidate uses zigzag residuals. Restricting the trial to
sampled fixed-Rice actions restored exact 12/16-bit and q100 parity without
plane, depth, quality, or tile-geometry exceptions.

The final six-trial focused matrix was:

| Quality | Threads | Variant | Ratio | Encode | Decode | Bitrate |
|---:|---:|---|---:|---:|---:|---:|
| 90 | 1 | EXP-0078 | 4.685392x | 68.153 MP/s | 66.444 MP/s | 151.062880 Mb/s |
| 90 | 1 | block pack | 4.809339x | 66.398 MP/s | 69.938 MP/s | 147.169656 Mb/s |
| 90 | 4 | EXP-0078 | 4.685392x | 186.416 MP/s | 155.629 MP/s | 151.062880 Mb/s |
| 90 | 4 | block pack | 4.809339x | 181.645 MP/s | 162.438 MP/s | 147.169656 Mb/s |
| 100 | 1 | EXP-0078 | 2.743688x | 63.314 MP/s | 60.428 MP/s | 257.969880 Mb/s |
| 100 | 1 | block pack | 2.743688x | 64.855 MP/s | 60.761 MP/s | 257.969880 Mb/s |
| 100 | 4 | EXP-0078 | 2.743688x | 176.627 MP/s | 147.228 MP/s | 257.969880 Mb/s |
| 100 | 4 | block pack | 2.743688x | 170.472 MP/s | 139.113 MP/s | 257.969880 Mb/s |

Thus matched q90 bytes fell 2.577%, one-thread encode throughput fell 2.575%,
and decode throughput improved 5.259%. Y PSNR remained 52.001930 dB, luma
block SSIM remained 0.99373056, and maximum error remained four. q100 was
byte-identical and exact. The charged model and identical realized bytes
show that 720/2160 matched q90 tiles (33.33%) selected packing.

Across the four-sample native supplement at q90, summed bytes fell 1.998%.
The geometric mean encode change was -0.820% at one thread and -0.975% at
four; decode improved 3.169% and 2.480%, respectively. The 12-bit UI and
16-bit motion streams were byte-identical to EXP-0078. Aggregate q100 bytes
were identical; one-thread encode/decode changed +1.698%/+1.140% and
four-thread changed -2.876%/-2.144%. Every recorded reconstruction metric was
identical between variants.

The GOP-12 single-frame access matrix read 20,238 fewer bytes for each
10-bit q90 dependency interval and exactly the same bytes in every q100 and
16-bit cell. Geometric-mean useful-throughput changes were -1.366% (10-bit
q90), -1.736% (10-bit q100), -0.702% (16-bit q90), and +0.817% (16-bit
q100), all inside the 5% timing tolerance.

The new malformed width, padding, truncation, and trailing-byte checks pass.
Lean builds the fixed-width fitting lemma. Strict Clippy and formatting pass.
The isolated unified-speed branch passes 50/55 library tests; its five
maximum-policy failures are the same expected EXP-0078 assertions for fixed
gradient and omitted 8-bit rANS selection. All block-pack, q100 exactness,
loss bound, malformed-stream, independent-mode decode, metric, and model
tests pass.

Artifacts:

- final source commit: `23801ae`;
- final release binary:
  `artifacts/frontier/fastvid-speed-exp0086-block-pack`
  (`9a9b156f7e941a7b701b18e21fbc03175086e47025188a0ddf3e872686ee1877`);
- final native supplement:
  `artifacts/exp0086-block-pack-highbit-final.tsv`
  (`90378fe6ffe4b6e0d93263ab44c664e91ad72a6f5c2c011cd0ecb1cf9ae48941`);
- single-frame access:
  `artifacts/exp0086-block-pack-access.tsv`
  (`4c48f3f22e59649633f7131ba74cb2f746df990699228a135c9ce24e4fc25a72`);
- focused unrestricted prototype:
  `artifacts/exp0086-block-pack-focused.tsv`
  (`8893b519548e42896d0438399c7517c2e6d4db72741f5eea027e2663a4263d83`);
- selector audit:
  `artifacts/exp0086-block-pack-selector.tsv`
  (`6830ab4a3ef1a334bd1039e1352451e0d8fbed7ba1dc16c6916f385127106f92`);
- audit harness:
  `scripts/audit-block-pack-selector.sh`
  (`36b7db6a6c0f53ddec606093da92f38b1195b6a397622462b836962750fbb452`).

## Decision

Accept the scalar high-bit mode and its fixed-Rice sampled selector. It passes
every declared gate, improves rate and decode throughput, preserves quality,
and has bounded access impact. The content-derived restriction fixes a real
model-family mismatch without encoding plane identity or a corpus-tuned
threshold.

Promote it as the successor speed-frontier artifact under the standard 5%
timing tolerance: versus EXP-0078 it materially improves q90 bytes and decode
while encode remains within tolerance. Preserve EXP-0078 in its immutable
record. The new scalar block decoder is now a justified target for portable
word-at-a-time or SIMD unpacking, but any kernel must retain scalar dispatch,
malformed-input validation, and byte-identical syntax.

## References

- [Research 0034](../research/0034-block-bitpacking-kernels.md)
- [EXP-0077](EXP-0077-high-bit-prefix-rice-streaming.md)
- [EXP-0085](EXP-0085-block-bitpacking-model.md)
