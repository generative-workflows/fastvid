# EXP-0085 — Charged block bit-packing model

Status: **ACCEPTED**

## Classification

**Speed/format exploration** — model a 128-symbol fixed-width entropy mode
grounded in research 0034 before implementing syntax or SIMD.

## Hypothesis

Fixed clamp-gradient residuals may have locally bounded magnitudes even when
tile-wide Rice codes retain variable unary work. Charging one control byte
and one fixed width per 128-symbol block could keep the matched q90 complete
stream within 3% of EXP-0078 while winning enough tiles to justify a fast,
specialized bit-packing mode.

## Model

Build a read-only native-high-bit model that exactly reproduces the fixed
clamp-gradient residuals and current zero-run/best-Rice bytes, then:

1. partition each tile's folded residual sequence into blocks of 128;
2. derive the minimum unsigned bit width from each block maximum;
3. charge one full control byte per block;
4. charge `ceil(symbol_count * width / 8)` data bytes per block;
5. compare complete bit-pack and current payload bytes per tile; and
6. report a hypothetical tile-local hybrid that selects the smaller complete
   payload with no uncharged side stream.

Existing directory entropy-mode storage can identify a tile mode, so no extra
tile selector byte is charged. No outlier patching is modeled.

## Gate

Advance a normative scalar mode only when:

- matched q90 bit-pack-only stream increase is at most 3%, or the charged
  hybrid reduces bytes by at least 1%;
- at least 10% of matched q90 tiles choose bit packing;
- aggregate q90/q100 hybrid bytes do not regress;
- q90 reconstruction/error exactly matches current fixed gradient and q100
  remains exact by construction; and
- no sample's hybrid stream grows.

The model is a rate gate only. Passing it does not establish speed.

## Result

The current-entropy control exactly reproduced the EXP-0078 matched q90
stream at 18,882,860 bytes and its established error boundary. Charged
bit-pack-only and tile-hybrid results were:

| Sample | Quality | Pack-only change | Hybrid change | Pack wins |
|---|---:|---:|---:|---:|
| 10-bit HDR gradient | 90 | +35.88% | -2.57% | 72/216 |
| 12-bit precision UI | 90 | +110.38% | 0.00% | 0/216 |
| 10-bit precision motion | 90 | +35.91% | -2.58% | 720/2160 |
| 16-bit precision motion | 90 | +168.86% | 0.00% | 0/2160 |
| 10-bit HDR gradient | 100 | +38.63% | 0.00% | 0/216 |
| 12-bit precision UI | 100 | +146.01% | 0.00% | 0/216 |
| 10-bit precision motion | 100 | +38.71% | 0.00% | 0/2160 |
| 16-bit precision motion | 100 | +153.99% | 0.00% | 0/2160 |

Aggregate q90 hybrid savings were 2.00% with 792/4752 tiles (16.67%)
selecting bit packing. q100 selected current entropy for every tile and was
byte-neutral. The hypothetical hybrid can never grow by construction and
reconstruction is identical because the predictor/quantizer is unchanged.

Every win occurred on the Cr plane: 72/72 HDR-gradient Cr tiles and 720/720
matched-motion Cr tiles. Y and Cb never selected packing, and neither the
12-bit UI nor 16-bit motion sample selected it. The concentration is both an
implementation opportunity and a serious generalization warning. A static
`Cr + 10-bit + q90` policy would be corpus-bound and is not acceptable.

The model passed strict Clippy and formatting. It charges a full control byte
per 128 symbols, all packed data bytes, frame headers, and tile directories;
it models no exception stream.

Artifacts:

- complete matrix:
  `artifacts/exp0085-block-pack-model.tsv`
  (`f3c7399304a518f363663dc40042e41af409a6442cc1caddfb79f2e2898ad3e5`);
- release model binary:
  `target/release/block_pack_model`
  (`40028c2b2aafa061ad944f04e4ab0800d20bc6b3fa52eaf8a231fe30a32bcb7b`);
- model source:
  `src/bin/block_pack_model.rs`
  (`ed5f3b219b1c59258903911ff1647a5323fe9cf477128fa17d2717b2a4c26ab4`);
- corpus harness:
  `scripts/benchmark-block-pack-model.sh`
  (`42524dd70d4319df567e698d19941a60d8b81818d097c802abb5bf1f01856985`).

## Decision

Accept as format-prototype evidence, not as a frontier result. The charged
hybrid passes every declared rate gate and identifies a meaningful subset,
but its plane/content concentration forbids a hard-coded selector.

Advance one isolated prototype with:

- a deterministic source-row selector that charges the 128-symbol control;
- a scalar normative 128-symbol packer/decoder;
- unchanged clamp-gradient reconstruction and quality;
- exact fallback to EXP-0078 for non-selected tiles; and
- focused plus complete-supplement rate/speed evidence.

Do not implement outlier patching or architecture intrinsics yet. The scalar
format must first establish that selection and extra block buffering do not
erase the modeled rate win or the speed objective.

## References

- [Research 0019](../research/0019-modern-integer-entropy-kernels.md)
- [Research 0034](../research/0034-block-bitpacking-kernels.md)
- [EXP-0038](EXP-0038-byte-oriented-residual-model.md)
- [EXP-0079](EXP-0079-unified-speed-profile.md)
