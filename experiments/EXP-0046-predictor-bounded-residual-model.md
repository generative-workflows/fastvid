# EXP-0046 — Predictor-bounded residual mapping model

Status: **PENDING**

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

## References

- [Research 0023: forward-citation review of space-saving prediction and
  symbol models](../research/0023-forward-citation-space-savings.md)
- [Research 0005: adaptive Rice residual
  coding](../research/0005-adaptive-rice-coding.md)
- [EXP-0038: byte-oriented residual format
  modeling](EXP-0038-byte-oriented-residual-model.md)

