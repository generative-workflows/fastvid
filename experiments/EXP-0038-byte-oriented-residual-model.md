# EXP-0038 — Byte-oriented residual format modeling

Status: **REJECTED**

## Hypothesis

A Stream-VByte-style control/data layout, especially its zero-aware 0/1/2/4
variant, is small enough on a meaningful subset of Fastvid residual tiles to
justify an exact SIMD encode/decode prototype without materially sacrificing
compression.

## Modification

Add a read-only analyzer that parses an already encoded Fastvid tile payload,
recovers its normative folded residual symbols, and computes:

- actual selected Rice/zero-run payload bytes;
- standard Stream VByte bytes: one 2-bit control per symbol plus 1/2/3/4
  little-endian data bytes;
- zero-aware Stream VByte 0/1/2/4 bytes: the same fully charged control stream
  plus 0/1/2/4 data bytes.

The analyzer does not change encoder decisions, decoder behavior, directory
bytes, or the bitstream. The existing entropy-mode byte can identify a future
mode, so no additional hypothetical tile header is charged; all control and
payload bytes are charged.

## Test

1. Unit-test modeled sizes at every byte-width boundary, partial four-symbol
   groups, all-zero runs, and both Rice and zero-run source payloads.
2. Require the analyzer to reject malformed streams through the normal parser
   and to consume exactly the declared residual count.
3. Run all 18 standard 8-bit corpus samples at qualities 60, 75, 90, 95, and
   100, preserving per-frame and aggregate rows.
4. Run the native 10/12/16-bit supplement at qualities 90 and 100, preserving
   bit-depth groups.
5. Report tile win rates, total modeled size deltas, percentiles, and results
   by content type, prediction mode, entropy mode, quality, and bit depth.

## Prototype gate

An implementation experiment is justified only if at least one byte-oriented
variant satisfies either:

- total modeled payload expansion no greater than 2% and at least 20% of
  tiles are at least 5% smaller; or
- at least one predeclared content/bit-depth group is at least 5% smaller in
  aggregate without another standard group expanding more than 5%.

Failing this gate rejects a format-level SIMD prototype. Small reusable LUT or
control/data-write kernels may still be studied independently if they preserve
the existing stream exactly.

## Results

The matrix produced 164,664 tile rows across:

- all 18 standard 8-bit samples at qualities 60, 75, 90, 95, and 100;
- native 10-, 12-, and 16-bit samples at qualities 90 and 100;
- GOP 1 stills and GOP 12 videos, with every video frame retained.

Artifact: `artifacts/exp0038-entropy-model.tsv` (164,665 lines including the
header), SHA-256
`29c7bd7560b0d8955d59e90089fb5a9162752fd642390b982b760d39f7fd105c`.

Command:

```text
scripts/benchmark-entropy-model.sh
```

| Variant | Actual payload | Modeled payload | Aggregate delta | Tiles >=5% smaller | Per-tile oracle delta | Gate A | Gate B |
|---|---:|---:|---:|---:|---:|---|---|
| Stream VByte 1/2/3/4 | 756,278,847 | 3,999,477,847 | +428.84% | 0 / 164,664 | 0.00% | Fail | Fail |
| Stream VByte 0/1/2/4 | 756,278,847 | 2,012,735,565 | +166.14% | 9 / 164,664 (0.01%) | less than 0.005% | Fail | Fail |

No standard Stream VByte tile was smaller. The zero-aware variant had only
nine 5%-winning tiles, all temporal zero-run tiles from
`procedural-scene-cuts`, concentrated in frames 3, 4, and 7 at q90--q100.
Their individual savings were too small to move the corpus total when chosen
by a per-tile oracle.

The most favorable aggregate groups still expanded:

| Variant | Best group | Delta |
|---|---|---:|
| 1/2/3/4 | 10-bit high-precision motion | +94.54% |
| 0/1/2/4 | 16-bit high-precision motion | +39.56% |

The natural-camera spot check was even less favorable: on
`camera-cholla` q90, standard Stream VByte expanded 223.08% and 0/1/2/4
expanded 113.72%, with no winning tile.

The analyzer binary SHA-256 was
`d072e11e07071bc2470e0640948cdb18572145dd996e516d20913b2d74e3ca2a`.
All 28 release tests, strict Clippy, formatting, and Lean passed. Current
8-bit q90 and 12-bit q90 streams were byte-identical to the preserved accepted
EXP-0032 binary.

## Conclusion

Reject a Stream-VByte format or SIMD kernel prototype. The fixed quarter-byte
control cost and byte-granular residual representation are fundamentally
mismatched with Fastvid's sub-byte Rice residuals and compact zero-run mode.
Even the best high-bit group missed the 5% group gate by a wide margin.

Retain the read-only analyzer and scripts: they provide an exact,
bitstream-neutral way to charge future entropy proposals and prevent
implementation effort based on misleading raw integer-throughput claims.
Further entropy work must preserve the current bitstream or target a denser
bit-level kernel.

## References

- [Research 0019](../research/0019-modern-integer-entropy-kernels.md)
- [EXP-0034](EXP-0034-perf-samply-cache-profile.md)
