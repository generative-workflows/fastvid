# EXP-0046 — Predictor-bounded residual mapping model

Status: **REJECTED**

## Hypothesis

The decoder-known predictor and quantization step bound every residual to an
asymmetric interval. Replacing symmetric zigzag with a bijection over that
interval will reduce exact selected Rice/zero-run payload bytes enough to
justify a per-tile residual-mapping mode, without changing reconstruction,
quality, directory length, or random access.

## Modification

Add a read-only analyzer for both 8-bit and native 10/12/16-bit paths. For
each source tile it must:

1. reproduce the encoder's spatial or temporal predictor and exact quantizer;
2. derive the attainable quantized interval
   `lo = quantize(-prediction, step)` through
   `hi = quantize(max_sample - prediction, step)`;
3. map the quantized residual bijectively from `[lo, hi]` into
   `[0, hi-lo]`, keeping zero at zero and alternating signs only while both
   signs remain possible;
4. run the existing Rice-parameter and zero-run byte selection over those
   mapped symbols; and
5. report current payload bytes, bounded-map payload bytes, and a per-tile
   oracle that retains the current mapping whenever it is no larger.

This experiment does not modify normal encoding, decoding, stream versions,
or current output bytes. A future format can signal the mapping in an existing
directory byte, so the model charges no new directory length. Whole-stream
results must nevertheless retain the complete current header and directory
when calculating percentages.

## Correctness tests

- Exhaustively prove by test, for every 8-bit prediction and quality step,
  that all encoder-produced quantized residuals are in the derived interval,
  mapping and inverse mapping round-trip, zero maps to zero, and codes are at
  most `hi-lo`.
- Cover asymmetric intervals in both directions, equal-sided intervals,
  interval endpoints, and the 10/12/16-bit maxima.
- Compare analyzer current-map bytes and entropy-mode decisions with normal
  encoded tiles.
- Preserve accepted 8-bit and 12-bit stream hashes.
- Pass release tests, strict Clippy, formatting, and Lean.

## Corpus test

Run the standard corpus matrix used by
[EXP-0038](EXP-0038-byte-oriented-residual-model.md):

- every standard 8-bit still and video at qualities 60, 75, 90, 95, and 100;
- GOP 1 for stills and GOP 12 for video;
- the native 10/12/16-bit supplement at qualities 90 and 100; and
- one thread, because this is a byte model rather than a throughput test.

Retain per-tile rows and report actual, always-bounded, and per-tile-oracle
payload bytes by bit depth, quality, content category, prediction type, and
entropy mode. Record tile win rates and distribution percentiles.

The predeclared, disjoint content categories are:

- `natural-cinema`: `bbb-*` and `ed-*`;
- `camera`: `camera-*` and `noisy-camera-*`;
- `ai-generated`: `ai-*`;
- `synthetic-ui`: `ui-*`, `procedural-*`, `resolution-*`, and
  `high-precision-ui-*`;
- `hdr-gradient`: `hdr-gradient-*`; and
- `high-precision-motion`: `high-precision-motion-*`.

## Prototype gate

Implement a format mode only if the per-tile oracle reduces total complete
stream bytes by at least 2% on the combined corpus and at least 1% in two or
more predeclared content categories. No category may expand because the old
mapping remains selectable. The always-bounded result is diagnostic and is
not required to win every category.

If the aggregate gate fails, retain the analyzer and proceed to the compatible
multi-predictor oracle from research 0023. If it passes, implement and
benchmark the bounded mapping before adding more predictors so later
prediction candidates are evaluated under the denser symbol model.

## Results

The complete matrix produced 164,664 tile rows across 880 encoded frames:

- all 18 standard 8-bit samples at qualities 60, 75, 90, 95, and 100;
- the native 10/12/16-bit supplement at qualities 90 and 100;
- GOP 1 stills and GOP 12 video; and
- the exact current zero-run/Rice selector for both mappings.

Artifact: `artifacts/exp0046-residual-mapping-model.tsv`, SHA-256
`6b3f1971a7312a0e462cee93a52776b2d280fe0fbc6803f470be765f3cc87f45`.
The analyzer binary SHA-256 was
`ee5b85cdc29c095da100dac566517b8e901d3e032ed59d09da972bf12e055706`.

Command:

```text
scripts/benchmark-residual-mapping-model.sh
```

| Measure | Current | Always bounded | Per-tile oracle | Oracle change |
|---|---:|---:|---:|---:|
| Tile payload bytes | 756,278,847 | 755,248,220 | 755,248,220 | -0.14% |
| Complete stream bytes | 761,576,255 | 760,545,628 | 760,545,628 | **-0.14%** |

The bounded representation never expanded a modeled tile: it was smaller for
59,786 / 164,664 tiles (36.31%) and byte-identical for the rest. Only 3,659
tiles (2.22%) saved at least 5%, and the median tile delta was zero. Because
the always-bounded and oracle totals are identical, retaining zigzag per tile
does not improve the result.

Complete payload changes by predeclared category were:

| Category | Change |
|---|---:|
| natural cinema | -0.08% |
| camera | -0.08% |
| AI-generated | -0.09% |
| synthetic/UI | -0.26% |
| HDR gradient | -0.04% |
| high-precision motion | -0.24% |

By precision, changes were -0.12% at 8-bit, -0.31% at 10-bit, -0.09% at
12-bit, and -0.13% at 16-bit. Spatial tiles saved 0.16% and temporal tiles
0.12%. Zero-run source tiles benefited more than Rice tiles, but still only
by 0.28% versus 0.12%.

The complete-stream result misses the 2% aggregate gate by more than an order
of magnitude, and no category reaches the 1% secondary threshold.

All 35 release tests passed, including exhaustive mapping/inverse tests for
every 8-bit encoder interval, high-bit endpoint intervals, and analyzer
agreement with real current payload lengths and entropy decisions. Strict
Clippy, formatting, and the Lean build passed. Normal encoding and decoding
were not modified; the established 8-bit and 12-bit stream controls remain
`474eea3b68bdbfa0c4f133699fa3dc0a17aa1ff6658b1afa489e96cd05c2eac8`
and
`d82e90e8229597c0acd19676de4b5ccd8f8f147fb651f2e1778643168432c29f`.

## Conclusion

Reject a format-level predictor-bounded mapping mode. The mapping is exact,
free of tile expansion, and mathematically halves the worst-case symbol
alphabet, but Fastvid's selected Rice and zero-run codes already assign most
probability mass to small magnitudes. Boundary-tail symbols occur too rarely
for the reduced alphabet to matter in encoded bytes. A new mode and inverse
kernel are not justified for 0.14% complete-stream savings.

Retain the read-only analyzer and exact bijection tests. They provide a useful
symbol-cost primitive for future prediction experiments, but the next
compression exploration moves to the compatible per-tile predictor oracle
from research 0023 rather than implementing this mapping.

## References

- [Research 0023: forward-citation review of space-saving prediction and
  symbol models](../research/0023-forward-citation-space-savings.md)
- [Research 0005: adaptive Rice residual
  coding](../research/0005-adaptive-rice-coding.md)
- [EXP-0038: byte-oriented residual format
  modeling](EXP-0038-byte-oriented-residual-model.md)
