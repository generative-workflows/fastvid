# EXP-0071 — Charged chroma-from-luma model

Status: **REJECTED**

## Classification

**Compression exploration** — test a cross-plane predictor family that is
absent from every current frontier version.

## Hypothesis

A tile-local affine chroma-from-luma predictor, charged one DC byte and one
signed alpha byte per selected tile, will reduce aggregate chroma payload by
at least 2% on the standard development corpus. At least two content
categories must show positive savings.

## Modification

Add analysis-only model tooling for 8-bit YUV 4:2:2:

1. Encode and decode luma with the current maximum codec to obtain the exact
   reconstructed luma available to a decoder.
2. Horizontally combine each reconstructed luma pair and subtract the
   tile-local luma mean.
3. For each Cb/Cr tile, evaluate signed alpha values from -16 through 16 in
   1/8 steps around a signaled tile-local chroma DC byte.
4. Quantize and entropy-code candidate residuals using the exact current
   zero-run/Rice/rANS machinery.
5. Charge two control bytes and compare against the exact current chroma tile
   payload.

Do not change the encoder, decoder, format, frontier, or reconstructed output.

## Test

- q90 and q100 model runs on every applicable standard-corpus 4:2:2 sample;
- exact payload accounting against current encoded chroma tiles;
- deterministic alpha/DC selection and bounded arithmetic controls;
- report per-plane, per-category, aggregate chroma, and whole-stream savings;
- report selected-tile count so a few outliers cannot hide broad regressions.

## Gate

- aggregate chroma payload savings at least 2%;
- positive savings in at least two diverse content categories;
- candidate bytes include both control bytes;
- no category loses more than 1% if an oracle selector retains the current
  exact fallback;
- advance only to a format experiment, not directly to frontier promotion.

## Result

The fast-feedback screen used the first frame of four deliberately different
development samples at q90 and q100:

- `camera-cholla` (camera photograph);
- `ai-greenhouse` (AI-generated detail);
- `ui-dashboard-scroll` (synthetic UI);
- `procedural-scene-cuts` (synthetic graphics).

The exact charged oracle selected 15 of 984 chroma tiles. All 15 selections
were in `ai-greenhouse`; camera and synthetic/UI content selected none.

| Group | Selected tiles | Chroma saving | Whole-stream saving |
|---|---:|---:|---:|
| AI-generated | 15 / 288 | 0.632% | 0.234% |
| Camera | 0 / 288 | 0.000% | 0.000% |
| Synthetic/UI | 0 / 408 | 0.000% | 0.000% |
| **Total** | **15 / 984** | **0.360%** | **0.109%** |

Cb accounted for 14 selections and 0.633% chroma savings; Cr accounted for
one selection and 0.036%. The q90 and q100 chroma savings were 0.531% and
0.264%, respectively. Because the oracle always retains the exact current
payload, no group regressed.

Command:

```text
scripts/benchmark-chroma-model.sh
```

Artifact:

- `artifacts/exp0071-chroma-screening.tsv`
- SHA-256
  `bc19cd4e7a925b5431b21ec7fcee62c8c94278e12bff93a6b9ec976a7399464c`

The release test suite passed all 54 library tests plus the two motion-model
tests.

## Decision

Reject this predictor formulation. Its 0.360% aggregate chroma saving is far
below the predeclared 2% gate, and only one content category benefits. Per the
evaluation methodology, the fast tier is a rejection tool; spending a
complete-corpus run on a candidate already this far below both gates would not
increase decision confidence enough to justify the CPU cost.

The result does not reject all cross-component prediction. It specifically
rejects a tile-wide affine model with one source-mean DC byte and one signed
alpha byte on already converted YUV 4:2:2. Future work would need a materially
different model, such as boundary-derived local parameters or chroma-to-chroma
prediction, and must receive a new experiment number.

## References

- [Research 0032](../research/0032-chroma-from-luma-prediction.md)
- [EXP-0048](EXP-0048-tile-predictor-format.md)
- [EXP-0055](EXP-0055-modeled-rans-selector.md)
- [EXP-0068](EXP-0068-four-state-rans.md)
