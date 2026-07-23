# EXP-0042 — Cached Rayon tile scheduler

Status: **REJECTED**

## Hypothesis

Replacing the per-call scoped-thread, atomic-index, mutex-output scheduler with
cached Rayon work-stealing pools and ordered indexed collection will improve
multi-thread encode/decode throughput, especially across video frames, while
preserving exact tile order, streams, quality, and compression.

## Modification

Add one shared internal parallel-map implementation:

1. retain the ordinary sequential iterator when the effective worker count is
   one;
2. lazily cache Rayon pools by worker count, capped by
   `available_parallelism()` and the number of tasks;
3. run an indexed range through `into_par_iter().map(...).collect()` inside
   the selected pool;
4. remove the duplicated per-tile atomic counter, output `Mutex`, and scoped
   OS-thread spawning from both codec modules.

Pool creation occurs during the existing warm-up and is excluded from recorded
steady-state trials. No format, prediction, entropy, reconstruction, or
public data-model change is allowed.

## Test

1. Add scheduler tests for empty, single-thread, capped, ordered, and repeated
   pooled calls.
2. Require fresh standalone candidate provenance and byte-identical 8-bit and
   12-bit controls against EXP-0041.
3. Run the balanced 8-bit video fast matrix at four threads first.
4. Advance only if its encode or decode geomean improves at least 5% with no
   unexplained cell regression beyond 3%.
5. Confirm on 8-bit stills and the native high-bit supplement at one/four
   threads; the one-thread path must stay inside 3%.
6. For acceptance, compare cycles, instructions, task-clock, context switches,
   and migrations on a multi-frame four-thread case where the PMU exposes
   credible counts.
7. Run release tests, strict Clippy, formatting, Lean, and exact-stream
   controls.

## Acceptance criteria

- Output ordering and every encoded stream are exact.
- Four-thread video encode or decode geomean improves at least 5%.
- No confirmed affected cell regresses more than 3%.
- One-thread throughput remains inside 3%.
- Counters or symbolized profiles support reduced scheduler coordination.
- The dependency and transitive licenses remain MIT/Apache-compatible.

## Results

All 33 release tests, strict Clippy, formatting, scheduler order/reuse tests,
and exact-stream controls passed. The fresh candidate SHA-256 was
`70b5f0bc7d3ddf71d9e8f4efbb34a40004ad9dd631afa7a10bd53cc5601810c0`,
distinct from EXP-0041. The 8-bit and 12-bit controls retained hashes
`474eea3b68bdbfa0c4f133699fa3dc0a17aa1ff6658b1afa489e96cd05c2eac8`
and
`d82e90e8229597c0acd19676de4b5ccd8f8f147fb651f2e1778643168432c29f`.

The balanced four-trial 8-bit video/four-thread fast matrix measured:

- encode geomean: **-7.02%**;
- encode cell range: -16.65% to +0.08%;
- decode geomean: **-4.59%**;
- decode cell range: -11.46% to -0.28%.

The candidate regressed nearly every content/quality cell. Longer sequences
already amortize scoped OS-thread creation, each tile contains substantial
codec work, and the original scheduler takes its output mutex only once per
completed tile. Rayon's recursive range splitting and indexed collection did
not offset those costs on this four-core host.

Artifact: `artifacts/exp0042-fast-8bit-video-t4.tsv`, SHA-256
`d145cc9c7bcebc7eb5d732abd78abed35de226607de9154734df5388fdbeccf5`.

## Conclusion

Rejected at the fast gate. Rayon parallel iterators are not a drop-in
throughput improvement for Fastvid's coarse tile map. The result does not show
that persistent workers are useless; it shows that replacing the proven
atomic dynamic queue with recursive iterator scheduling is counterproductive.
A follow-up should retain one atomic tile assignment queue, use persistent
workers, and return worker-local result batches so it isolates thread creation
and output-lock removal from scheduling-policy changes.

## References

- [Research 0021](../research/0021-rayon-work-stealing.md)
- [EXP-0041](EXP-0041-fresh-build-rice-emission-correction.md)
