# EXP-0051 — High-bit staged predictor set

Status: **ACCEPTED**

## Classification

**Exploitation of the EXP-0048 compression candidate** — remove predictor
work that did not contribute to the measured high-bit space frontier.

## Hypothesis

For high-bit content, evaluating Paeth and clamp-gradient on every tile,
average on intra tiles, and temporal when available will preserve the
EXP-0048 corpus sizes within 0.1% while reducing selector work enough to bring
the focused encode slowdown closer to the compression-frontier gate.

## Predeclared model evidence

In EXP-0047's 9,504 high-bit tile rows:

- half-gradient won zero tiles;
- average won 79 tiles, all in frame zero;
- Paeth, clamp-gradient, and temporal account for every other winner.

Static subset selection therefore matches the high-bit oracle exactly on the
recorded rows. This is a corpus-derived encoder heuristic, not a bitstream
restriction: every version-2 prediction mode remains specified and decodable.

## Modification

- High-bit inter tiles evaluate Paeth, clamp-gradient, and temporal.
- High-bit intra tiles evaluate Paeth, clamp-gradient, and average.
- Do not evaluate half-gradient in the production high-bit selector.
- Keep the 8-bit exhaustive selector and all decoder modes unchanged.

## Correctness tests

- Candidate high-bit sizes stay within 0.1% of EXP-0048 actual/oracle sizes.
- q100 exactness and lossy maximum-error bounds remain unchanged.
- Every version-2 mode remains independently decodable.
- Release tests, strict Clippy, formatting, and Lean pass.

## Measurement

1. Run focused four-trial high-bit q90 video A/B.
2. Compare candidate sizes with EXP-0048 high-bit artifacts.
3. If promising, run the complete high-bit still/video matrix and
   single-frame access confirmation.

## Acceptance gate

- Combined high-bit bytes regress no more than 0.1% from EXP-0048.
- Focused high-bit encode throughput improves at least 15% relative to the
  EXP-0049 candidate.
- No quality or error-bound regression.
- Decode behavior is unchanged; the separate EXP-0049 result remains a known
  issue for spatial zero runs.

## Result

The four-trial focused q90 video comparison preserved both encoded sizes
exactly and improved encode throughput relative to EXP-0049:

- 10-bit candidate/baseline encode ratio: `0.2179x` to `0.2878x`;
- 16-bit candidate/baseline encode ratio: `0.1789x` to `0.2519x`;
- focused geometric mean: `0.1974x` to `0.2692x`, a 36.4% relative
  improvement and a reduction from 5.07x to 3.71x slowdown.

The complete four-trial q90/q100 high-bit confirmation matched all eight
EXP-0048 candidate sizes byte for byte: 70,345,876 bytes in both versions.
Across those cells, candidate encode throughput was `0.3701x` the balanced
baseline geometric mean, or a 2.70x slowdown. Decode geometric mean was
`0.9836x`; the known 16-bit spatial-zero-run outlier remains and is handled
separately.

The six-trial 8-bit fast-feedback control was unchanged by this high-bit-only
selector. It measured 8-bit encode throughput at `0.3005x` baseline geometric
mean, or a 3.33x slowdown, with the expected EXP-0048 sizes.

Artifacts:

- `artifacts/exp0051-highbit-q90-video-ab.tsv`
  (`f52b004c9d52ca053f5020722a3eeda516abdd8a024485e1a24e77965614feee`);
- `artifacts/exp0051-highbit-stills-ab.tsv`
  (`3705ba366487c05272ec33e3fa214f4460a4de8c98caae1687f22dc9a1260352`);
- `artifacts/exp0051-highbit-video-ab.tsv`
  (`1293d0c2647188062035f442fb7b6db536bc5ddd5533208103bbf710f306acb0`);
- `artifacts/exp0051-fast-feedback.tsv`
  (`ff77338ebfad17ab756b6df221ad512863f1d446877754602c780851f1adba96`).

Candidate binary:

- `artifacts/frontier/fastvid-compression-exp0048`
  (`8a273da4ac54cf646c8d54c5f9581ed5d6ab8c8279a08a9beab81c10fc790a09`).

## Decision

Accept. The corpus-derived staged search preserves the measured compression
frontier exactly, improves focused high-bit encode throughput by more than
twice the predeclared gate, and brings both focused and full high-bit encoder
slowdowns inside the compression-frontier allowance. Keep decoder support for
all five modes; this acceptance applies only to the production high-bit
encoder search policy.
