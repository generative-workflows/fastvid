# EXP-0132 — Retire rejected model tooling

Status: **ACCEPTED**

## Classification

**Engineering cleanup** — reduce the active CPU-to-CUDA handoff surface
without changing a stream format or discarding experimental evidence.

## Hypothesis

Rejected one-off model binaries and their dedicated benchmark/summarizer
scripts can be retired from the active tree while:

- every production and frontier encoder still builds;
- all emitted stream versions still decode;
- the current v5 high-bit frontier encoder emits byte-identical output;
- the full release test and lint suites pass; and
- immutable experiment and research records continue to identify the retired
  work and its measurements.

This should remove at least four compiled binary targets and 1,500 lines of
inactive implementation and harness code.

## Modification

Remove the standalone implementations and scripts for:

- EXP-0071's rejected chroma-from-luma model;
- EXP-0075's rejected reversible-squeeze model;
- EXP-0081's rejected above-only predictor model;
- EXP-0065's rejected block-motion-potential model; and
- EXP-0131's rejected adaptive-MED block model.

Remove the rejected chroma model's read-only library analysis API and data
type. Retain:

- all immutable experiment and research records;
- every decoder for legacy and current 8-bit and high-bit stream versions;
- the v2 production and v5 full-tile parallel high-bit encoder paths;
- the v4 bounded-shard comparison path while it remains a tested format
  branch;
- corpus, metrics, profiling, frontier, access, and OpenAPV tooling; and
- current entropy, predictor, residual-mapping, and block-pack models.

Add a short lifecycle policy to `experiments/README.md` so future rejected
one-off tooling is not mistaken for permanent production surface.

## Test

- audit non-record references to every retired symbol, binary, and script;
- run `cargo fmt --check`;
- run release tests for all targets;
- run strict debug and release Clippy for all targets;
- build the retained binary targets;
- syntax-check every retained shell script;
- encode the checksummed HDR 10-bit fixture at q90/one thread with the current
  v5 high-bit frontier encoder and compare its stream hash with the EXP-0130
  control;
- inspect the final diff and line/target counts.

## Result

The audit found no active source, script, evaluation, or benchmark reference
to a retired symbol after excluding the immutable experiment and research
records. The cleanup reduces Cargo's automatically discovered binary targets
from 13 to 9 and removes 1,798 tracked lines. The deleted code comprises four
standalone model binaries, four benchmark drivers, three summarizers, the
rejected chroma analysis API/model type, and its unit test. EXP-0131's
uncommitted one-off model and harness were also discarded rather than added
to the active tree.

The retained surface is:

- `fastvid` and `corpusgen`;
- `encode16_profile`;
- entropy, predictor, residual-mapping, and block-pack model tools;
- the block-pack kernel benchmark and SSIM-sampling model; and
- 8-bit v0/v2/v3 plus high-bit v1/v2/v4/v5 decoding.

All 69 release library tests and all nine binary test targets pass. Strict
debug and release Clippy, formatting, documentation, retained shell syntax,
and diff checks pass. The v5 q90 HDR control is byte-identical to EXP-0130:

```text
9a3cf708ecdc73f9f8c15a545b41f761ad1ed844c2b8cb4db42118ce587fce37
```

The source fixture hash is
`64a17c11e49fe4f2d0a6afe5316e3b38e4bc7dd17b0cf5e2cfbeb502f503879a`;
the encoded stream remains 1,735,875 bytes.

## Decision

Accept the conservative cleanup. It removes inactive experimental compile
surface without changing production/frontier output or decoder coverage.
Keep the v4 encoder comparison path for now: unlike the removed standalone
models, it is an emitted format branch with active round-trip and malformed
stream tests and provides a bounded-predictor baseline for v5. Reassess
encoder-only v4 retirement separately after the GPU implementation defines
which CPU frontier branches remain useful.

## References

- [EXP-0065](EXP-0065-block-motion-potential.md)
- [EXP-0071](EXP-0071-chroma-from-luma-model.md)
- [EXP-0075](EXP-0075-charged-reversible-squeeze-model.md)
- [EXP-0081](EXP-0081-above-predictor-screen.md)
- [EXP-0131](EXP-0131-adaptive-med-block-model.md)
