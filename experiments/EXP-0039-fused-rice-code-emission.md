# EXP-0039 — Fused exact Rice code emission

Status: **REJECTED**

## Hypothesis

Packing the unary quotient separator and fixed-width remainder into one
bounded `u64` append when the complete Rice code fits will reduce work in the
profiled `finish_entropy` hotspot and improve encode throughput without
changing one bit of output.

## Modification

In the 8-bit and high-bit `BitWriter::put_rice` paths:

1. compute quotient, remainder, and total code length once;
2. when the code plus the writer's at-most-seven buffered bits fits in `u64`,
   form the exact existing LSB-first bit pattern and append it once;
3. retain the current unary-zero plus separator/remainder fallback for long
   codes.

No decoder, format, parameter selection, payload length, or entropy-mode
decision changes. This is arithmetic fusion rather than a lookup table: the
8-bit full table would consume tens of KiB and a 16-bit table would be
cache-hostile.

## Test

1. Exhaustively compare fused and reference writers for every 8-bit folded
   value and parameter, every high-bit boundary value and parameter, every
   initial bit alignment, and long-code fallback cases.
2. Require byte-identical encoded streams and reconstruction signatures
   against the preserved EXP-0032 binary.
3. Run the balanced fast feedback matrix first. Advance only if encode
   geomean improves at least 2% with no cell regression beyond 3%.
4. If it advances, run the full high-bit supplement and repeat the EXP-0034
   PMU profile to determine whether `finish_entropy`, cycles, or instructions
   improve.
5. Run release tests, strict Clippy, formatting, Lean, and 8-bit/high-bit
   exact-stream controls.

## Acceptance criteria

- Every tested stream is byte-identical.
- Fast encode geomean improves at least 2%.
- No encode cell regresses more than 3%; decode remains inside the 5% gate.
- Any accepted wall-time result is supported by reduced instructions/cycles
  or reduced `finish_entropy` sample share.

## Results

Release tests, strict Clippy, formatting, exhaustive 8-bit writer
equivalence, and high-bit boundary/fallback equivalence passed. Encoded
streams were byte-identical to EXP-0032 for the 8-bit camera and native
12-bit UI controls:

- 8-bit q90: `474eea3b68bdbfa0c4f133699fa3dc0a17aa1ff6658b1afa489e96cd05c2eac8`;
- 12-bit q90: `d82e90e8229597c0acd19676de4b5ccd8f8f147fb651f2e1778643168432c29f`.

The balanced four-trial high-bit fast matrix produced:

- encode geomean: **+1.02%**;
- decode geomean: +0.54%;
- encode cell range: -3.88% to +10.67%;
- decode cell range: -3.54% to +5.31%.

The candidate missed the 2% encode advancement threshold and exceeded the
-3% per-cell regression limit on 12-bit UI q100 in both thread modes and on
16-bit motion q90/four-thread. The large positive and negative multithreaded
cells also show that this small local change is not reliably distinguishable
from scheduling and allocation noise at those timings.

Artifact: `artifacts/exp0039-fast-gop1.tsv`, SHA-256
`188c791588401b1e038c5d117d8cc8218e421d978b8eaa32386edd3dc67e1182`.
Candidate binary SHA-256:
`8e9043efadd68e90ba2ad301279d711664cf91177018de14b8e9360a1316d04b`.

## Conclusion

Rejected and reverted at the fast gate. Fusing Rice code construction is
correct and occasionally faster, but its aggregate gain is too small and
unstable to retain. Slow PMU confirmation was intentionally skipped. Future
work should reduce the number of residual traversals or bytes touched in
`finish_entropy`, where a larger instruction/cache effect is plausible,
rather than further micro-optimizing the final bit append.

## References

- [Research 0019](../research/0019-modern-integer-entropy-kernels.md)
- [EXP-0034](EXP-0034-perf-samply-cache-profile.md)
- [EXP-0038](EXP-0038-byte-oriented-residual-model.md)
