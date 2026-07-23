# EXP-0025 — Current headline benchmark refresh

Status: **ACCEPTED**

## Hypothesis

The README headline table is stale after accepted encoder and decoder
optimizations and should be replaced with measurements from the current
implementation rather than supplemented with experiment-by-experiment prose.

## Modification

No codec change. Run the corpus-v2 codec track at q90/q100, one/four threads,
and GOP 1 using the current release binary. Replace the README benchmark
section with a compact present-state summary and move historical detail to the
experiment records.

## Test

1. Warm every sample/settings cell and record one development trial.
2. Preserve all 18 per-sample rows per quality/thread setting.
3. Report geometric-mean compression, arithmetic-mean MP/s and raw MB/s, and
   arithmetic-mean quality metrics, matching the previous headline convention.
4. Verify thread-invariant encoded sizes and quality.

## Acceptance criteria

- Every headline number is derived from the new artifact.
- Encoded sizes and quality match between thread counts.
- README contains current state, not a chronological optimization narrative.

## Results

Current corpus-v2 GOP-1 development snapshot:

| Quality | Threads | Geo. ratio | Encode | Decode | Y PSNR | SSIM |
|---:|---:|---:|---:|---:|---:|---:|
| 90 | 1 | 7.308x | 38.43 MP/s | 56.13 MP/s | 49.868 dB | 0.996557 |
| 90 | 4 | 7.308x | 141.52 MP/s | 177.67 MP/s | 49.868 dB | 0.996557 |
| 100 | 1 | 5.013x | 32.27 MP/s | 50.72 MP/s | exact | 1.000000 |
| 100 | 4 | 5.013x | 131.62 MP/s | 179.44 MP/s | exact | 1.000000 |

Raw decimal throughput is exactly derived from actual plane sizes and is
reported in the README. Encoded sizes, PSNR, SSIM, and maximum error matched
between thread counts for every sample. Full rows are in the ignored artifact
`artifacts/headline-current.tsv`.

The README now contains only the current implementation, current snapshot,
measurement definitions, documentation links, and current limitations.
Optimization history remains in immutable experiment records.

## Conclusion

Accepted. The previous headline table was stale and has been replaced; the
README is no longer used as a chronological experiment log.


## References

- [Evaluation methodology](../EVALUATION_METHODOLOGY.md)
- [EXP-0008](EXP-0008-corpus-v2-expansion.md)
- [EXP-0021](EXP-0021-entropy-allocation-final.md)
- [EXP-0024](EXP-0024-quantizer-table-confirmation.md)
