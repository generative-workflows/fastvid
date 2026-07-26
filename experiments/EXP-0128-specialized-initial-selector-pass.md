# EXP-0128 — Specialized initial selector pass

Status: **REJECTED**

## Classification

**Version-5 selector exploitation** — reduce the 36.21% fused selection
hotspot identified by EXP-0127.

## Hypothesis

Only the first paired Rice traversal evaluates parameters zero/one and charges
zero-run cost, but the current generic loop tests `parameter == 0` for every
symbol in every later pair. A dedicated initial pass can use the identities
`q0 = value` and `q1 = value >> 1`, charge zero-run cost without a mode
condition, then hand parameters two through sixteen to the simpler generic
pair loop.

This should preserve candidate order, convex termination, zero-run cost, and
all bytes while improving geometric encode throughput by at least 1.05x over
EXP-0126 and retaining at least 0.95x decode throughput.

## Modification

- evaluate parameters zero and one in a specialized first traversal;
- accumulate zero-run cost directly in that traversal;
- apply the existing exact candidate/termination checks in order;
- begin the generic adjacent-pair loop at parameter two with no zero-cost
  condition in its inner loop.

Do not alter the cost formulas, selected parameter, tie behavior, emission,
or syntax.

## Test

- retain exhaustive full-scan Rice equivalence and fused-zero cost oracles;
- retain the accepted version-5 control and all native q90 bytes/metrics;
- run five balanced whole-codec trials against the exact EXP-0126 binary;
- require at least 1.05x geometric encode and 0.95x geometric decode;
- run the full release suite, both strict Clippy configurations, formatting,
  and diff checks.

## Result

The exhaustive Rice oracle and fused zero-run cost oracle remain exact, and
every native q90 output byte and bitrate is unchanged.

Five balanced whole-codec trials measured:

| Sample | Candidate encode | Encode ratio | Decode ratio |
|---|---:|---:|---:|
| HDR gradient 10 | 38.113 MP/s | 1.052x | 1.011x |
| Precision motion 10 | 39.063 MP/s | 1.037x | 1.013x |
| Precision UI 12 | 42.506 MP/s | 1.035x | 0.995x |
| Precision motion 16 | 52.641 MP/s | 1.007x | 1.010x |
| **Geometric** | — | **1.0327x** | **1.0073x** |

The candidate improves the measured encoder by 3.27%, but that is inside the
methodology's 5% timing tolerance and fails the declared gate. The effect is
largest on the HDR still and negligible on 16-bit motion, so it is not broad
enough to justify duplicated hot-loop code.

The fixed EXP-0126 binary has SHA-256
`739e68994d7a04c602967f8fee0d09d001821dd2551293c769c8d211e8d67f29`;
the candidate has SHA-256
`17e9fbdc800dc27a33ae8edb8bec64030871ec17017d7e311fcb493f0b2b2d66`.
The balanced artifact is
`artifacts/exp0128-specialized-initial-confirm.tsv`
(`eb021ef0c69d8b4de4c508f7a5396c46fbe469ad196b78413d2e9ff5494c2103`).
Targeted exactness tests and strict normal Clippy pass.

## Decision

Reject and revert the specialized loop. The profile's selection share is
dominated by useful lane and zero-run cost accumulation rather than the
per-symbol parameter condition.

Future selector work should reduce arithmetic or traversals—such as sharing
more quotient-derived statistics—not duplicate the same work with a
specialized control-flow shape.

## References

- [EXP-0115](EXP-0115-convex-rice-search-bound.md)
- [EXP-0117](EXP-0117-paired-rice-parameter-pass.md)
- [EXP-0126](EXP-0126-selector-fused-zero-run-cost.md)
- [EXP-0127](EXP-0127-post-zero-cost-profile.md)
