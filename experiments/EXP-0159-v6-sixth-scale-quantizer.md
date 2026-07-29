# EXP-0159 — Version-6 sixth-scale quantizer

Status: **PENDING FULL CORPUS**

Implementation revision: `0c3bec7`

## Hypothesis

The q90 RGB10 Butteraugli failure is caused by the coarse integer step jump
from 5 to 9. Replacing the five-quality bucket mapping with a sixth-scale
mapping will select step 8 at q90, clear both perceptual gates, and retain the
strict latency gates with less rate cost than q95.

## Modification

Version new encoder output to Fastvid v6 and derive its quantizer as:

`step = 1 + ceil((100 - quality) * 2^(bit_depth - 8) / 6)`

The decoder accepts both versions: v5 retains its original bucketed mapping,
while v6 uses the new mapping. This preserves existing v5 streams instead of
silently reinterpreting their quality byte. The public inspector accepts v5
and v6. Prediction, entropy coding, metrics, and `scripts/evaluate.py` remain
unchanged.

## Canonical evaluation

Frozen evaluator artifacts (5 warm-ups, 20 repetitions, FFVShip 5.0.0-a):

- failing v5 q90 control: `/tmp/perf-rans4-hybrid2-q90.json`;
- v6 q90 control: `/tmp/perf-v6-step6-q90.json`;
- v6 q90 required matrix: `/tmp/v6-step6-matrix-q90.json`;
- v6 q100 required matrix: `/tmp/v6-step6-matrix-q100.json`.

## Result

On the RGB10 1920x1080 hard control, SSIMULACRA2 rose from 94.940620 to
96.121964 and Butteraugli fell from 1.050937 (fail) to 0.786318 (pass). Median
encode was 0.880128 ms and decode was 0.436736 ms, both inside the strict
1.0/0.5 ms gates. The frame grew from 2,092,542 to 2,437,430 bytes, but remains
6.84% smaller than the quality-qualified q95 rANS frame (2,616,088 bytes).

All nine q90 matrix samples passed with minimum SSIMULACRA2 93.179863 and
maximum Butteraugli 0.795349. All nine q100 samples passed exactly, with
minimum SSIMULACRA2 99.811905 and Butteraugli 0.0. A preserved 2,905,962-byte
v5 RGB stream decoded successfully under the v6-capable decoder.

## Decision

Retain as the current quality-qualified mapping. Full acceptance remains
pending because the checked-in canonical full manifest, normalized 100–200
image 4K corpus, and 4K x24 throughput inputs required by INSTRUCTIONS.md are
not present. The temporary matrix cannot prove those full-scope gates.

## References

- [EXP-0158](EXP-0158-cuda-four-lane-rans.md)
- [EXP-0027](EXP-0027-high-bit-quantizer-table.md)
- [EXP-0151](EXP-0151-corpus-v4-full-frame-feedback.md)
