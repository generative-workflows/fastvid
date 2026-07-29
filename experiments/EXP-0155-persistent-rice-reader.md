# EXP-0155 — Persistent CUDA Rice reader and decode geometry cache

Status: **ACCEPTED**

## Hypothesis

Retaining a per-lane Rice bit buffer across symbols will avoid repeated scalar
bit extraction and reduce the RGB10 1920x1080 q95 entropy kernel by at least
25%. Removing buffers that are completely overwritten and caching immutable
tile geometry after warm-up will then bring canonical decode latency below the
0.5 ms gate without changing bytes or pixels.

## Modification

- Add one persistent 64-bit LSB-first reader per Rice lane.
- Refill it bytewise only as consumed bits leave space; use `ffs` for unary
  quotient runs and direct masking for remainders.
- Allocate folded residual, output, and device-parsed shard metadata without
  redundant zero fills; every element is written before use.
- Cache immutable tile and parser metadata by CUDA device, layout, dimensions,
  and tile geometry. Entropy metadata parsing and reconstruction still execute
  for every decode.

The bitstream, encoder, quantizer, quality, and canonical evaluator are
unchanged.

## Test

- Profile the q95 RGB10 1920x1080 control.
- Run the unchanged canonical rejection evaluator with 5 warm-ups and 20
  repetitions.
- Round-trip every required format/depth cell at q90 and q100; require exact
  q100 pixels and deterministic streams.
- Reject five malformed CUDA-resident streams covering directory mutation,
  broken offsets, unknown entropy mode, truncation, and trailing data.

## Result

`decode_shards_kernel` fell from 421.507 us to 235.521 us (-44.1%). Total
profiled CUDA time fell from 633.764 us to 444.736 us (-29.8%). The unchanged
canonical evaluator measured RGB10 1920x1080 q95 decode at 0.498224 ms, down
from 0.740160 ms (-32.7%), clearing the strict 0.5 ms latency gate. Encode was
0.798448 ms, also below its 1.0 ms gate. Encoded size remained 2,905,962 bytes;
SSIMULACRA2 remained 96.772240 and Butteraugli remained 0.778911.

All eight required format/depth cells passed deterministic q90 and exact q100
round trips. All five malformed streams were rejected.

Canonical candidate artifact: `/tmp/perf-rice-cache-q95.json`.

## Decision

Accepted. The candidate creates decode budget for a denser entropy fallback
without changing rate or quality. The 128-thread reconstruction sub-screen was
neutral (183.809 us versus 184.545 us) and was not retained.

## References

- [Research 0039](../research/0039-parallel-rice-bitstream-hardware.md)
- [Research 0042](../research/0042-gpu-variable-output-assembly.md)
- [EXP-0146](EXP-0146-cuda-device-metadata-parse.md)
- [EXP-0154](EXP-0154-word-rice-decoder.md)
