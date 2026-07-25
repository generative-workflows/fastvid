# Runtime-invariant integer division

## Question

Can exact multiply-and-shift division replace Fastvid's dependent quantizer
lookup without restoring a hardware divide?

## Sources

- Lemire, Bartlett, and Kaser,
  [*Integer Division by Constants: Optimal Bounds*](https://arxiv.org/abs/2012.12369),
  2020.
- [libdivide](https://libdivide.com/), maintained reference implementation
  and documentation.
- [`strength_reduce` 0.2.4](https://docs.rs/strength_reduce/0.2.4/strength_reduce/),
  safe Rust implementation, MIT OR Apache-2.0.

These are implementation or primary research sources. The two software
sources have permissive licenses compatible with Fastvid's MIT target.

## Findings

Compilers replace division by a compile-time constant with a multiply,
optional correction, and shift, but Fastvid's quantization step is selected
at runtime. libdivide and `strength_reduce` precompute the equivalent divider
state once, then reuse it across a hot loop. libdivide specifically notes
that unsigned division is the favorable case and that results depend on
compiler loop unswitching and scheduling.

Fastvid already moves signed division out of the loop with an exact lookup
table. EXP-0027 measured that table 14.03% faster than ordinary runtime
division. The new profile changes the comparison: the table load is dependent
on the causal predictor result and the 16-bit table is 524,284 bytes.
Strength reduction exchanges that load for independent integer arithmetic.
It may therefore win even though both methods already avoid `idiv`.

Quantization is sign-magnitude rounding:

`q = sign(r) * ((abs(r) + step / 2) / step)`.

The magnitude and divisor are positive and bounded to 65,535 and 4,865
respectively under the current format, so the crate's exact unsigned
division applies without approximation. Exhaustive equality with the scalar
oracle remains mandatory.

## Experimental boundary

Test a safe Rust strength-reduced candidate directly rather than infer from
an isolated arithmetic loop. The fused predictor is causal, and instruction
scheduling against prediction and entropy work is the claimed benefit.
Reject the dependency if the matched q90 encode path gains less than 2%.

Do not adopt libdivide's C/C++ implementation: it conflicts with Fastvid's
preference against C/C++ dependencies, while the Rust implementation tests
the same algorithmic direction.

## Relevant experiments

- [EXP-0027](../experiments/EXP-0027-high-bit-quantizer-table.md)
- [EXP-0090](../experiments/EXP-0090-post-pack-speed-profile.md)
- [EXP-0093](../experiments/EXP-0093-proven-quantizer-lookup.md)
- [EXP-0094](../experiments/EXP-0094-strength-reduced-quantizer.md)
