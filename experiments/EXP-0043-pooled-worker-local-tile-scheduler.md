# EXP-0043 — Pooled worker-local tile scheduler

Status: **REJECTED**

## Hypothesis

EXP-0042 changed thread lifetime, work assignment, and collection at once and
regressed. Keeping Fastvid's efficient atomic dynamic tile queue while using a
cached Rayon pool and worker-local result batches will isolate the two
user-identified coordination costs: repeated OS-thread creation and the
per-tile output mutex.

## Modification

Reuse EXP-0042's cached pools, but replace indexed parallel iteration with one
pool `broadcast` job per worker. Every worker:

1. claims tiles from the existing relaxed atomic counter;
2. computes them sequentially into a private `Vec<(index, value)>`;
3. returns that batch without shared output writes.

The calling thread places batches into one ordered `Vec<Option<T>>` after all
workers finish. This preserves dynamic load balancing and exact output order,
removes per-tile mutex locking, and reuses workers across frames. The
single-thread path remains unchanged.

## Test

1. Retain EXP-0042 scheduler order, cap, and pool-reuse tests.
2. Require fresh candidate provenance and exact 8/12-bit stream controls.
3. Run the same four-thread 8-bit video matrix used by EXP-0042.
4. Advance only for at least 5% encode or decode geomean improvement with no
   unexplained cell regression beyond 3%.
5. Confirm an advancing candidate on stills, high-bit, one-thread controls,
   and scheduler counters.
6. Run release tests, strict Clippy, formatting, Lean, and license checks.

## Acceptance criteria

- Output order, streams, quality, and compression are exact.
- Four-thread video encode or decode geomean improves at least 5%.
- No confirmed affected cell regresses more than 3%.
- One-thread throughput stays inside 3%.
- Counters support reduced creation/coordination overhead.

## Results

All 33 release tests, strict Clippy, formatting, scheduler order/reuse tests,
and exact 8/12-bit stream controls passed. The fresh candidate SHA-256 was
`7ce8c83af0e871466f68d9e489abf765bff6c2479d655c3d5ec749b9f3e56ee0`.

The same balanced four-trial 8-bit video/four-thread matrix as EXP-0042
measured:

- encode geomean: **-0.49%**;
- encode cell range: -3.26% to +1.33%;
- decode geomean: **-0.22%**;
- decode cell range: -3.46% to +3.12%.

Retaining the atomic dynamic queue recovered EXP-0042's large regression, but
neither persistent workers nor worker-local output batches produced a
material gain. The result is centered on zero and misses the 5% advancement
threshold. The small negative outliers occur in otherwise unchanged codec
kernels and do not justify slow confirmation.

Artifact: `artifacts/exp0043-fast-8bit-video-t4.tsv`, SHA-256
`a57fed1179f9d032f11fa3ed4cadad7fae573ee7454e2deb56df9346112fa2ad`.

## Conclusion

Rejected and reverted at the fast gate. On the current 256x128 tiles, useful
codec work amortizes scoped thread creation and the one-lock-per-tile output
handoff. Rayon's pool machinery is neutral only when Fastvid's existing
atomic scheduler is retained and substantially slower when parallel
iterators replace it. The dependency is removed rather than adding six
transitive packages without measured benefit. Revisit pooling only with
smaller tiles, more cores, frame-level parallelism, or profiles showing
scheduler symbols as material hotspots.

## References

- [Research 0021](../research/0021-rayon-work-stealing.md)
- [EXP-0042](EXP-0042-cached-rayon-tile-scheduler.md)
