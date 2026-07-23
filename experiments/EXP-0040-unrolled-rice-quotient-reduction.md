# EXP-0040 — Unrolled Rice quotient reduction

Status: **REJECTED**

## Hypothesis

The parameter-outer high-bit Rice selector retained after EXP-0028 repeatedly
reduces a tile's folded values into one `u64` quotient sum. Replacing the
single reduction dependency with four independent accumulators may expose
more instruction-level parallelism and a clearer auto-vectorization kernel,
reducing `finish_entropy` time without increasing its working set or changing
the stream.

## Modification

Keep the accepted parameter order and quotient-zero early termination. For
each parameter, reduce `chunks_exact(4)` into four independent `u64`
accumulators, combine them after the loop, and handle the at-most-three-value
tail. Do not change residual storage, entropy decisions, payload emission, or
the decoder.

This deliberately avoids the sample-outer 17-accumulator loop rejected by
EXP-0028 and the larger dual-representation code rejected by EXP-0035.

## Test

1. Compare the unrolled quotient kernel with the scalar reference over the
   complete folded domain, mixed vectors, every parameter, and all tail
   lengths.
2. Require byte-identical 8/10/12/16-bit stream controls against EXP-0032.
3. Run the balanced four-trial high-bit fast matrix first.
4. Advance only if encode geomean improves at least 2%, no cell regresses
   more than 3%, and decode remains within 5%.
5. For an advancing candidate, run the six-trial confirmation and repeat the
   EXP-0034 one-thread PMU profile for instructions, cycles, and L1D events.
6. Run release tests, strict Clippy, formatting, and Lean.

## Acceptance criteria

- Quotient sums, Rice choices, encoded streams, and reconstruction signatures
  are exact.
- Fast and confirmation encode geomeans improve at least 2%.
- No encode cell regresses more than 3%; decode stays inside the 5% gate.
- An accepted wall-time result is supported by reduced cycles/instructions or
  a reduced `finish_entropy` sample share.

## Results

The complete-domain quotient oracle, mixed tail lengths, 29 release tests,
strict Clippy, and formatting passed. A fresh standalone release build was
preserved after `cargo build --release`; its SHA-256
`2054fcfee5a74f67aa8ba7cdb3834f5a0e46a51cafd9df306d6c6666a30d47db`
differs from the EXP-0032 baseline
`512f345f01b235d92e9f5bd03ac7da6e4dde06ee8a0c02894f2a077e9ea45eec`.
The native 12-bit q90 control remained byte-identical with SHA-256
`d82e90e8229597c0acd19676de4b5ccd8f8f147fb651f2e1778643168432c29f`.

The balanced four-trial high-bit fast matrix produced:

- encode geomean: **-1.22%**;
- decode geomean: -0.44%;
- encode cell range: -13.81% to +2.80%;
- decode cell range: -6.30% to +5.11%.

The candidate missed both the aggregate advancement threshold and per-cell
gate. The largest encode regression was native 12-bit UI q100/four-thread;
the one-thread and video cells were mixed around zero, providing no evidence
of a stable kernel improvement.

Artifact: `artifacts/exp0040-fast-gop1.tsv`, SHA-256
`34cbd7ebccf90fcda59b0371caefe27886705845cff7557e9564daf7a575c67c`.

## Conclusion

Rejected and reverted at the fast gate. Four explicit scalar accumulators do
not improve the compiler's existing reduction and can worsen generated code
or binary layout. Slow PMU confirmation was skipped. A future Rice-cost
kernel should first establish a different generated-instruction pattern
rather than assuming source-level unrolling creates useful SIMD or ILP.

## References

- [Research 0020](../research/0020-modern-parallel-codec-kernels.md)
- [EXP-0028](EXP-0028-single-pass-high-bit-rice-cost.md)
- [EXP-0029](EXP-0029-rice-cost-early-termination.md)
- [EXP-0034](EXP-0034-perf-samply-cache-profile.md)
- [EXP-0035](EXP-0035-narrow-folded-residuals.md)
