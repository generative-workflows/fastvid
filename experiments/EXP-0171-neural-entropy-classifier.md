# EXP-0171 — Neural entropy classifier

Status: **SUPERSEDED**

Date: 2026-07-29

## Hypothesis

A fixed sub-64 KiB integer network classifying each residual shard into static
entropy distributions can improve rejection encoded bytes by at least 0.5%
without failing a canonical gate.

## Baseline

Code and evaluator revision: `763ef10442be860f1748e80c4aa16339ed410c21`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

```sh
PYTHONPATH=. python3 scripts/evaluate.py --tier rejection \
  --output /tmp/fastvid-exp0171-neural-entropy-baseline-rejection.json \
  --libvship-revision v5.0.0 \
  --libvship-build '5.0.0 CUDA direct C API' \
  --libvship-gpu-id 0 --libvship-workers 2
```

The baseline passed all 11 samples: 347,833,953 encoded bytes, 6.188000859x
compression, minimum SSIMULACRA2 93.697319, and maximum Butteraugli 0.084405.
All timing gates passed. Artifact:
`/tmp/fastvid-exp0171-neural-entropy-baseline-rejection.json`.

No candidate artifact exists and no codec source was changed.

## Decision

Supersede before implementation after the research target expanded to
potentially large gains around 10x. The classifier's 0.5% target cannot close
the measured 38% byte gap and the current per-shard order-0 model already uses
exact local symbol frequencies. Preserve the baseline as evidence, but prioritize
lossy frequency transform and spatial perceptual allocation research.

