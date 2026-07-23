# EXP-0004 — Per-tile adaptive zero-run/Rice entropy coding

Status: **ACCEPTED**

## Hypothesis

Dense small residuals cause the accepted zero-run syntax to spend one or two
whole bytes per sample. Selecting between zero-run varints and Rice parameters
0 through 8 independently per tile will reduce the quality-75 synthetic
fixture by at least 10%, preserve the sparse quality-100 size and decoded
pixels, and keep the dense-path throughput cost below 10% on one thread.

This follows the quality-75 anomaly in
[EXP-0002](EXP-0002-zero-run-tokens.md) and the
[adaptive Rice review](../research/0005-adaptive-rice-coding.md).

## Modification

- Build a histogram of zigzag-mapped quantized residuals per tile.
- Compute the exact Rice bit length for parameters 0 through 8.
- Select the smallest of those modes and the existing zero-run payload.
- Store the entropy selector in a formerly reserved directory byte, adding no
  payload overhead.
- Use buffered, least-significant-bit-first Rice input/output and require
  canonical zero padding.
- Add a Lean proof that a Rice quotient and remainder recompose their input.

## Test

Use the deterministic 1920x1080 YUV422p8 fixture at qualities 100 and 75.
Compare the committed EXP-0002 implementation against this change with five
warm release runs, plus seven interleaved runs at quality 75. Require exact
quality-100 round trips, unchanged quality metrics, malformed-mode/padding
rejection, the full Rust suite, strict Clippy, and the Lean model to pass.

## Results

Host: 4-vCPU AMD EPYC-Genoa VM, 7.6 GiB RAM, Rust 1.97.1. Timings below are
medians from the seven interleaved quality-75 runs; VM scheduling produces
substantial four-thread variance, so the one-thread comparison is the
acceptance measurement.

| Version | Quality | Threads | Size | Ratio | Encode | Decode | Y PSNR |
|---|---:|---:|---:|---:|---:|---:|---:|
| zero-run baseline | 75 | 1 | 799,313 B | 5.188x | 53.557 ms | 34.449 ms | 43.123 dB |
| adaptive Rice | 75 | 1 | 652,138 B | 6.359x | 57.120 ms | 33.550 ms | 43.123 dB |
| zero-run baseline | 75 | 4 | 799,313 B | 5.188x | 14.897 ms | 10.976 ms | 43.123 dB |
| adaptive Rice | 75 | 4 | 652,138 B | 6.359x | 16.270 ms | 11.316 ms | 43.123 dB |

The adaptive stream is 18.4% smaller (compression ratio improves 22.6%),
single-thread encoding is 6.7% slower, and single-thread decoding is 2.6%
faster. Quality 100 remains exactly 37,080 bytes and round-trips exactly
because sparse tiles retain zero-run mode. All eleven Rust tests, strict
Clippy, and both Lean proofs pass.

## Decision

Accept. It clears the compression and one-thread throughput gates while
leaving sparse payload size and reconstructed pixels unchanged. The remaining
four-thread timing variance and representative-corpus gap stay in scope for
[EXP-0003](EXP-0003-regression-corpus.md).
