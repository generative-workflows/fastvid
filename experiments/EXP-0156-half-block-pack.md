# EXP-0156 — 64-symbol block packing

Status: **REJECTED**

## Hypothesis

Halving block-pack groups from 128 to 64 symbols will localize bit widths
enough to reduce RGB10 q95 output by at least 0.5% without violating the
strict 1080p latency gates.

## Modification

Change the CUDA block-pack analyzer, emitter, and decoder from 128-symbol to
64-symbol groups. The evaluator, predictor, quantizer, and other entropy modes
remain unchanged.

## Test

Run the frozen canonical rejection evaluator on the same RGB10 1920x1080 q95
latency and small-batch control used by EXP-0155, with 5 warm-ups and 20
repetitions.

## Result

Encoded size fell from 2,905,962 to 2,904,442 bytes: 1,520 bytes or 0.0523%.
Median encode latency was effectively unchanged at 0.799264 ms versus
0.798448 ms. Median decode latency increased from 0.498224 ms to 0.505440 ms,
crossing the strict 0.5 ms gate. Quality was unchanged at SSIMULACRA2
96.772240 and Butteraugli 0.778911.

Canonical candidate artifact: `/tmp/perf-block64-q95.json`.

## Decision

Rejected and reverted. The compression gain is an order of magnitude below
