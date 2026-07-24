# EXP-0047 — Compatible per-tile predictor and inter/intra oracle

Status: **ACCEPTED**

## Classification

**Exploration** — causal prediction and tile-local inter/intra structure.
This is a distinct technique family from EXP-0046's residual-symbol mapping
and from EXP-0042--0045's scheduler/allocation exploitation.

## Hypothesis

Image regions favor different causal predictors, and a frame-global temporal
gate leaves spatially better tiles on the table. Selecting among a small set
of permissively sourced predictors per tile will reduce complete stream bytes
by at least 2% while retaining Fastvid's existing directory length, tile
random access, quantization error bound, and GOP dependency depth.

## Modification

Add a read-only oracle; do not change normal encoding or decoding. For every
tile, encode exact current zigzag residuals and current zero-run/Rice
selection under:

1. current Paeth;
2. average of reconstructed left and above samples;
3. WebP lossless `ClampAddSubtractFull(left, above, upper_left)`;
4. WebP lossless
   `ClampAddSubtractHalf(Average2(left, above), upper_left)`; and
5. previous-frame prediction whenever a reconstructed reference exists,
   even if the current frame-global gate rejects temporal coding.

All spatial candidates reset state at tile edges and predict from their own
reconstructed samples. For each candidate record exact payload bytes and
squared reconstruction error. The oracle minimizes payload bytes, breaking
ties in favor of the current encoder's choice and then the lower squared
error.

The current directory already has a prediction-mode byte, so the model charges
no extra directory length. Complete-stream totals retain all current header
and directory bytes.

## Correctness tests

- Every q100 candidate must reconstruct exactly at 8/10/12/16-bit.
- Every lossy candidate must retain the existing quantizer maximum-error
  bound.
- The current Paeth and temporal model must exactly match real current tile
  payload lengths and entropy choices.
- Predictor arithmetic must be exhaustively checked at 8-bit boundaries and
  at representative 10/12/16-bit extrema.
- Normal accepted 8-bit and 12-bit stream hashes must remain unchanged.
- Release tests, strict Clippy, formatting, and Lean must pass.

## Corpus test

Use the EXP-0046 matrix and its predeclared content categories:

- all 18 standard 8-bit samples at qualities 60, 75, 90, 95, and 100;
- GOP 1 stills and GOP 12 video;
- native 10/12/16-bit samples at qualities 90 and 100; and
- one thread, because this is an offline byte/quality oracle.

Retain per-tile rows. Report complete bytes, selected-mode frequency, wins,
and payload deltas by category, bit depth, quality, prediction mode, and
candidate. Report aggregate and category squared error relative to the
current encoder; q100 remains exact.

## Prototype gate

Implement a format candidate only if:

- complete-stream bytes fall at least 2% over the combined corpus;
- at least two predeclared categories improve at least 1%;
- aggregate squared error at each lossy quality does not increase more than
  1%, and no category increases more than 3%;
- q100 reconstruction remains exact; and
- at least 10% of winning tiles select a mode other than the current choice,
  so the result is not a marginal tie artifact.

If the gate passes, first implement the smallest subset of modes responsible
for at least 90% of oracle savings, then find a cheaper selection heuristic.
The exhaustive encoder is not a production design.

## Results

The fast independent-still screen passed before the video work was expanded:
15,624 tile rows across 64 frames reduced complete bytes by 7.76% and squared
error by 7.98%. This justified implementing independent current/oracle
reconstruction chains for an exact GOP-aware model.

The complete matrix then produced 164,664 tile rows across 880 frames:

- all 18 standard 8-bit samples at qualities 60, 75, 90, 95, and 100;
- the native 10/12/16-bit supplement at qualities 90 and 100;
- GOP 1 stills and GOP 12 video; and
- complete current and oracle reconstruction propagation through every GOP.

| Measure | Current | Oracle | Change |
|---|---:|---:|---:|
| Tile payload bytes | 756,278,847 | 654,542,750 | **-13.45%** |
| Complete stream bytes | 761,576,255 | 659,840,158 | **-13.36%** |
| Squared reconstruction error | 977,060,587,925 | 976,470,900,529 | -0.06% |

The oracle reduced 58,448 / 164,664 tiles (35.50%). It selected a different
mode on 44,826 tiles (27.22% of all tiles and 76.69% of winning tiles), well
above the 10% non-tie gate. Selected modes were:

| Mode | Tiles | Payload savings attributed to selected mode |
|---|---:|---:|
| Paeth | 63,314 | 28,817,867 (28.33%) |
| Average | 14,739 | 11,383,871 (11.19%) |
| Clamp gradient | 6,402 | 35,183,340 (34.58%) |
| Half gradient | 3,680 | 4,452,854 (4.38%) |
| Temporal | 76,529 | 21,898,165 (21.52%) |

Savings on Paeth and temporal selections are real cascade effects: earlier
oracle choices produce a different reconstructed reference, which can make a
later tile smaller without changing its named predictor. This is why removing
a low-frequency mode cannot be evaluated only from its direct tile bytes.

Every predeclared category advanced:

| Category | Payload change | SSE change |
|---|---:|---:|
| natural cinema | -10.27% | -13.65% |
| camera | -4.94% | -2.70% |
| AI-generated | -1.41% | -1.74% |
| synthetic/UI | -10.79% | -2.97% |
| HDR gradient | -17.99% | less than 0.01% |
| high-precision motion | -41.55% | less than 0.01% |

Eight-bit payload fell 8.50%; 10-, 12-, and 16-bit payload fell 24.71%,
39.02%, and 60.99%. The quality groups saved 7.47--18.17%. Quality 100
remained exact. No quality or category/quality SSE group regressed; the worst
reported delta was zero within integer totals.

Artifacts:

- `artifacts/exp0047-predictor-stills-screening.tsv`
  (`1d4e9127f4289895b7556d52728368e08401fb964544c281de6a3081599d0ae8`);
- `artifacts/exp0047-predictor-model.tsv`
  (`4162b78efe3a80d62618bdcc1789851e42e2fe4b6e4fda2a42aca2b01004986e`).

The analyzer binary SHA-256 was
`f1651b4e14b16aa4116c5b4e6bf5d4db400d3722619b3c152ac64a66cd6448e4`.
Commands:

```text
scripts/benchmark-predictor-screening.sh
scripts/benchmark-predictor-model.sh
```

All 41 release tests passed. They cover exact agreement with real current tile
payloads and entropy decisions, every 8-bit boundary pair, high-bit extrema,
the quantizer error bound, q100 exactness for every candidate, and exact
oracle reconstruction propagation. Strict Clippy, formatting, and Lean
passed. Normal encoding and decoding were not modified, so the established
8-bit and 12-bit stream controls remain unchanged.

## Conclusion

Accepted as an exploration result and implementation gate. Compatible
tile-local predictor selection is the first space branch in this pass to
produce a large, broad, quality-safe improvement. It exceeds the 2% complete
stream gate by more than 6x, improves all content categories, and is even more
valuable at high bit depths.

Proceed to a distinct format experiment. Preserve the EXP-0045 balanced line,
implement all four spatial candidates plus tile-local temporal selection as
the initial compression-frontier version, and measure its real encoder cost.
Then exploit the oracle artifact to derive a cheaper selector; do not treat
the five-pass analysis implementation as a production encoder.

## References

- [Research 0023: forward-citation review of space-saving prediction and
  symbol models](../research/0023-forward-citation-space-savings.md)
- [Research 0008: block-local inter/intra
  selection](../research/0008-block-local-inter-intra-selection.md)
- [EXP-0046: predictor-bounded residual
  model](EXP-0046-predictor-bounded-residual-model.md)
