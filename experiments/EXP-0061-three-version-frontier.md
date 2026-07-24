# EXP-0061 — Three-version frontier

Status: **ACCEPTED**

## Classification

**Evaluation-methodology correction** — keep the active technology portfolio
within the requested two-to-three representative versions after EXP-0060
filled the speed role.

## Hypothesis

Speed, practical compression, and maximum compression preserve the useful
extrema and middle of the measured frontier. The former balanced snapshot is
a weak knee after EXP-0060: compared with speed it gains only 7.93%
compression ratio while losing 20.68% encode throughput and 29.19% decode
throughput. Retiring it from active measurement will cut confirmation cost by
25% without deleting its source, binary, or experiment evidence.

## Modification

- Keep exactly three active registry entries: speed, practical compression,
  and maximum compression.
- Retain balanced as a historical reference in `FRONTIER.md`, Git, and its
  preserved local binary, but do not duplicate it as an active slot.
- Restore six cyclic trials: with three active binaries, each occupies each
  execution position twice.
- Regenerate the canonical frontier matrix, summary, and graph.

## Test

- Validate all three active binary hashes.
- Run four cases by three versions by six trials, one CPU-bound process at a
  time.
- Require stable encoded bytes and complete trials.
- Regenerate the summary and SVG twice and require byte-identical output.
- Confirm that no current-state document claims four active versions.

## Gate

Accept if the active registry contains exactly three distinct reproducible
versions, the matrix passes, and the generated graph retains the speed,
practical-compression, and maximum-compression points.

## References

- [EXP-0057](EXP-0057-automated-pareto-frontier.md)
- [EXP-0060](EXP-0060-fixed-gradient-speed-tier.md)

## Result

The hash-validated six-trial run produced the required 72 rows: four cases by
three versions by six trials. Every version occupied each execution position
twice per case, encoded bytes were stable, and graph validation passed.

| Active version | Compression | Encode MP/s | Decode MP/s | Playback bitrate |
|---|---:|---:|---:|---:|
| speed | 13.353556x | 122.803921 | 147.634583 | 68.853922 Mb/s |
| practical compression | 24.547776x | 28.938688 | 136.613715 | 37.455311 Mb/s |
| maximum compression | 33.613405x | 24.487733 | 96.387897 | 27.353510 Mb/s |

Two independent generations from the same raw matrix were byte-identical:

- raw matrix: `artifacts/exp0061-frontier.tsv`
  (`35bf1569d1d8396635a6718c83831e755552464d2493b9d41c9c66ee785dabd2`);
- graph: `benchmarks/frontier.svg`
  (`9290d6a8c7076b9058aeaab3409917dca117debdbf42d156710ea190943e03d2`);
- summary: `benchmarks/frontier-summary.tsv`
  (`ee57309e920904d6a256cfe71717a6334b74ff2d659640ab4d03eebd286a73af`).

Current-state documentation contains no claim of four active versions. The
balanced source, experiment, and local preserved binary remain available as
historical evidence.

## Decision

**Accepted.** The active portfolio now satisfies the requested two-to-three
version bound while retaining the throughput, practical-compression, and
maximum-compression representatives. Future candidates must replace one of
these three rather than expanding routine confirmation indefinitely.
