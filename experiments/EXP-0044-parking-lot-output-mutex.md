# EXP-0044 — `parking_lot` output mutex

Status: **REJECTED**

## Hypothesis

Replacing only `std::sync::Mutex` with `parking_lot::Mutex` in the original
atomic tile scheduler will reduce short critical-section coordination cost
without adding allocations, changing scheduling, or affecting output order.

## Modification

Retain the accepted pre-Rayon scheduler exactly:

- scoped OS workers per encode/decode call;
- one relaxed atomic tile index;
- one preallocated ordered `Vec<Option<T>>`;
- one output lock acquisition per completed tile;
- one final ordered collection.

Change only the mutex type and its non-poisoning lock/unwrap syntax in both
codec modules. Use `parking_lot` 0.12.5 with default features.

## Test

1. Require fresh binary provenance and exact 8/12-bit stream controls against
   EXP-0041.
2. Run the balanced four-trial 8-bit video/four-thread matrix used by
   EXP-0042 and EXP-0043.
3. Advance only for at least 2% encode or decode geomean with no unexplained
   cell regression beyond 3%.
4. Confirm an advancing result on high-bit, stills, one-thread controls, and
   scheduler counters.
5. Verify direct and transitive licenses.
6. Run release tests, strict Clippy, formatting, and Lean.

## Acceptance criteria

- Streams, ordering, quality, and compression are exact.
- Four-thread video encode or decode geomean improves at least 2%.
- No confirmed affected cell regresses more than 3%.
- One-thread throughput remains inside 3%.
- Dependency licenses are MIT/Apache-compatible.

## Results

All 30 release tests, strict Clippy, formatting, and exact-stream controls
passed. The fresh candidate SHA-256 was
`95c3a8f10b103acd09463471f1c8df56e91c03e7564ba2c3b14050aff11cbb63`.

The balanced four-trial 8-bit video/four-thread matrix measured:

- encode geomean: **+0.15%**;
- encode cell range: -1.28% to +2.13%;
- decode geomean: **-0.80%**;
- decode cell range: -3.71% to +1.28%.

The mutex substitution is centered on measurement noise and misses the 2%
advancement threshold. It produces neither a consistent encode benefit nor a
decode benefit. Since the lock is acquired only once after a complete
256x128-or-smaller tile has been predicted and entropy-coded, its documented
adaptive userspace fast path is too infrequent to affect total codec time.

Artifact: `artifacts/exp0044-fast-8bit-video-t4.tsv`, SHA-256
`6944037762eb09828dfb3585fb7ef74b8b19dde556ff48936dfc729b5042f51b`.

## Conclusion

Rejected and reverted at the fast gate. `parking_lot` is an appropriate
short-lock implementation, but Fastvid's current output lock is not a
throughput bottleneck. The dependency and its transitive packages are removed
rather than retained for a statistically negligible result. The original
scoped-thread, atomic-index, standard-mutex scheduler remains the accepted
implementation.

## References

- [Research 0022](../research/0022-parking-lot-mutex.md)
- [EXP-0042](EXP-0042-cached-rayon-tile-scheduler.md)
- [EXP-0043](EXP-0043-pooled-worker-local-tile-scheduler.md)
