# EXP-0041 — Fresh-build Rice emission correction

Status: **ACCEPTED**

## Hypothesis

EXP-0039's fused Rice-emission modification remains a plausible exact
bit-writer optimization, but its performance result is invalid because the
standalone CLI was copied after `cargo test --release` without an explicit
`cargo build --release`. The copied hash therefore represented an older
standalone binary. Repeating the experiment with a demonstrably fresh,
distinct candidate will determine the actual effect.

## Modification

Reapply EXP-0039's bounded fused Rice append to both 8-bit and high-bit
writers. When the unary quotient, separator, remainder, and current buffered
bits fit in `u64`, append the complete LSB-first code once; otherwise use the
existing long-code fallback.

Update the evaluation methodology to require an explicit standalone release
build, distinct baseline/candidate hashes for CLI-reachable changes, and an
exact-stream control hash before any A/B timing.

## Test

1. Repeat exhaustive 8-bit and high-bit boundary/fallback writer equivalence.
2. Run `cargo build --release`, preserve the standalone candidate, and require
   its SHA-256 to differ from both EXP-0032 and the stale EXP-0039 binary.
3. Require exact 8-bit q90 and 12-bit q90 stream hashes.
4. Run the balanced four-trial high-bit matrix.
5. Advance to six-trial confirmation and PMU profiling only for at least 2%
   encode geomean, no cell regression beyond 3%, and decode inside 5%.
6. Run release tests, strict Clippy, formatting, and Lean.

## Acceptance criteria

- Writer oracles and stream bytes are exact.
- Candidate provenance proves the standalone binary is fresh.
- Fast and confirmation encode geomeans improve at least 2%.
- No encode cell regresses more than 3%; decode stays inside 5%.
- An accepted result has supporting PMU or sampled-hotspot evidence.

## Results

All 30 release tests, strict Clippy, formatting, the exhaustive 8-bit writer
oracle, and the high-bit boundary/fallback oracle passed. The explicitly
rebuilt candidate SHA-256 was
`e4c0bd83c0acd9b7d962f81b191a1da8835692adef7a195bded3b487631931ed`,
distinct from both:

- EXP-0032 baseline:
  `512f345f01b235d92e9f5bd03ac7da6e4dde06ee8a0c02894f2a077e9ea45eec`;
- stale EXP-0039 standalone:
  `8e9043efadd68e90ba2ad301279d711664cf91177018de14b8e9360a1316d04b`.

Exact-stream controls retained their established hashes:

- 8-bit camera q90:
  `474eea3b68bdbfa0c4f133699fa3dc0a17aa1ff6658b1afa489e96cd05c2eac8`;
- native 12-bit UI q90:
  `d82e90e8229597c0acd19676de4b5ccd8f8f147fb651f2e1778643168432c29f`.

The high-bit four-trial fast matrix advanced with **+9.68%** encode geomean.
Its six-trial confirmation measured:

- encode geomean: **+10.32%**;
- encode cell range: -1.05% to +24.86%;
- decode geomean: -0.81%;
- decode cell range: -4.58% to +3.14%.

The four-trial 8-bit video matrix, which includes natural motion, noisy
camera, UI scrolling, and procedural cuts, measured:

- encode geomean: **+12.22%**;
- encode cell range: -1.79% to +35.55%;
- decode geomean: -0.54%;
- decode cell range: -3.31% to +3.53%.

The broad 8-bit image confirmation measured +7.64% encode geomean but included
short-job scheduling interruptions where unchanged decode time doubled
together with encode time. Ten-trial focused reruns resolved the affected
settings:

- q100/four-thread image geomean: **+9.48%**, range -0.33% to +19.35%;
- q90/one-thread image geomean: **+6.63%**, range -0.55% to +17.50%.

Repeated hardware counters on native 10-bit q90 motion, one thread, showed:

| Counter | EXP-0032 | Candidate | Change |
|---|---:|---:|---:|
| cycles | 4,472,097,791 | 4,222,362,759 | -5.58% |
| instructions | 16,417,567,740 | 16,229,327,902 | -1.15% |
| branches | 2,625,440,493 | 2,524,317,109 | -3.85% |
| branch misses | 35,949,952 | 26,779,463 | -25.51% |

The L1D miss alias returned an implausible zero in this multiplexed run and is
excluded rather than interpreted. Reduced cycles, instructions, branches, and
branch misses independently support the wall-time result.

Artifacts:

- `artifacts/exp0041-fast-gop1.tsv`
  (`236fe0d52f219448eea63d682abdfcf889b303ae646847c12cad1b64c7720fd9`);
- `artifacts/exp0041-confirm-gop1.tsv`
  (`9ecd8edb1017fde2ee70d350dd05ceff3c44b36c8892334277cd5dacda009c17`);
- `artifacts/exp0041-8bit-images-confirm-gop1.tsv`
  (`290bdc984543f39e34091c617a82ad3d738e25eaec11383a17a33663fea788b0`);
- `artifacts/exp0041-8bit-video-gop1.tsv`
  (`0aa20c3102496d25ca47d5e8a8892f84e2f42e6e5c3b7c305c129ed6294c7399`);
- `artifacts/exp0041-8bit-images-q100-t4.tsv`
  (`a6853777e1a47038b3703f3bcd347191a6378346b861870a59473e4cef471c40`);
- `artifacts/exp0041-8bit-images-q90-t1.tsv`
  (`e62194e9adbfef4c1fd0f6b2b9459277efe19bcb715e5276b01333362bc8f412`);
- `artifacts/exp0041-perf-baseline.txt`
  (`e0d9c90213e241add0a93f3eb589c0903ef6b844132eb8cb93993c8aff31adac`);
- `artifacts/exp0041-perf-candidate.txt`
  (`1caa4c615b8de186dfdf32cd2e17532fe0f8408e390777651309ddf6265ee1d4`).

## Conclusion

Accepted. A single bounded bit append substantially accelerates both 8-bit
and high-bit Rice emission while preserving every tested stream and metric.
The long-code fallback explains the deliberately small effect on lossy
16-bit motion; all other representative groups improve materially. The
evaluation loop now requires an explicit standalone release build and binary
provenance so stale executables cannot support future timing conclusions.

## References

- [EXP-0034](EXP-0034-perf-samply-cache-profile.md)
- [EXP-0039](EXP-0039-fused-rice-code-emission.md)
- [Evaluation methodology](../EVALUATION_METHODOLOGY.md)
