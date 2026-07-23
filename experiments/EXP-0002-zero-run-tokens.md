# EXP-0002 — Zero-run residual tokens

Status: **ACCEPTED**

## Hypothesis

The Paeth baseline produces long runs of zero quantized residuals on smooth
material. Encoding each zero run as one even-valued token and each nonzero
residual as one odd-valued token will break the one-byte-per-sample floor,
improve the deterministic 1080p fixture to greater than 2× compression, and
retain exact quality-100 reconstruction.

This directly follows the entropy bottleneck measured in
[EXP-0001](EXP-0001-paeth-varint-baseline.md).

## Modification

Keep all format and prediction choices fixed. Replace the tile payload mapping:

- token `2 × (run_length - 1)` represents `run_length` zero residuals;
- token `2 × zigzag(nonzero_residual) - 1` represents one nonzero residual;
- tokens use canonical unsigned LEB128.

## Test

Repeat EXP-0001's 1920×1080 quality/thread matrix. Require all existing
correctness and malformed-input tests to pass, and add malformed run-overflow
coverage.

## Results

Host and fixture match EXP-0001. Values are single-run development
measurements.

| Quality | Threads | Size | Ratio | Encode | Decode | Luma PSNR |
|---:|---:|---:|---:|---:|---:|---:|
| 100 | 1 | 37,080 B | 111.845× | 37.1 MP/s | 61.1 MP/s | exact |
| 100 | 4 | 37,080 B | 111.845× | 81.5 MP/s | 118.3 MP/s | exact |
| 90 | 1 | 18,580 B | 223.208× | 39.0 MP/s | 58.5 MP/s | 49.892 dB |
| 90 | 4 | 18,580 B | 223.208× | 96.0 MP/s | 143.7 MP/s | 49.892 dB |

The complete Rust test suite, strict Clippy, and release build passed. The
added malformed zero-run overflow case is rejected by the decoder.

## Decision

Accept the mechanism: it breaks the byte-per-sample floor, preserves exact
quality-100 output, and exceeds the experiment's 2× fixture gate. The very
large ratios reflect a highly predictable synthetic image and must not be
generalized. Representative-corpus work continues in
[EXP-0003](EXP-0003-regression-corpus.md).

