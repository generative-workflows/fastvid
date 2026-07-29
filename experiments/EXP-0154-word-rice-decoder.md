# EXP-0154 — Word-oriented CUDA Rice decoder

Status: **REJECTED**

## Hypothesis

The q95 RGB10 1920×1080 control spends 421.5 µs, or 66.5% of measured CUDA
time, in `decode_shards_kernel`. Most shards use four-lane Rice, whose decoder
currently reads every unary and remainder bit in a scalar loop. Loading a
bounded 64-bit window, counting unary zeros with `ffs`, and extracting the
remainder from one word will reduce the entropy kernel by at least 25% while
preserving decoded pixels and malformed-stream bounds checks.

## Modification

Replace scalar per-bit Rice reads with bounded little-endian word assembly.
The implementation never reads beyond its lane byte count, retains exact
bit-position and padding validation, and leaves the bitstream unchanged.

## Test

- Rebuild the CUDA extension.
- Round-trip every required format/depth cell at q90 and q100.
- Retain malformed-stream rejection.
- Profile and canonically evaluate the q95 RGB10 1920×1080 control.

## Gate

Accept only if decoded pixels remain identical, the entropy kernel improves by
at least 25%, and complete decode latency does not regress.

## Result

All required format/depth cells round-tripped at q90 and q100, including exact
q100 reconstruction. However, on the RGB10 1920x1080 q95 control,
`decode_shards_kernel` regressed from 421.507 us to 534.692 us (+26.9%).
Complete CUDA time increased from 633.764 us to 743.654 us. Reassembling a
bounded eight-byte window for every Rice symbol cost more than the scalar bit
reader it replaced.

## Decision

Rejected and reverted. A future word reader must retain state across symbols
within each lane rather than rebuilding the word window for every symbol.

## References

- [Research 0039](../research/0039-parallel-rice-bitstream-hardware.md)
- [EXP-0146](EXP-0146-cuda-device-metadata-parse.md)
