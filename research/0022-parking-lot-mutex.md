# `parking_lot` mutex for tile-output coordination

## Sources

- `parking_lot` 0.12.5 [crate documentation][crate].
- `parking_lot::Mutex` [API and standard-mutex comparison][mutex].
- `RawMutex` [implementation source][source].
- Upstream [repository][repo].

[crate]: https://docs.rs/parking_lot/0.12.5/parking_lot/
[mutex]: https://docs.rs/parking_lot/0.12.5/parking_lot/type.Mutex.html
[source]: https://docs.rs/parking_lot/0.12.5/src/parking_lot/raw_mutex.rs.html
[repo]: https://github.com/Amanieu/parking_lot

## Applicability

Fastvid's original tile scheduler performs substantial codec work outside the
lock, then takes one short output lock to place the completed tile in its
ordered slot. `parking_lot::Mutex` documents a one-byte lock state, an inline
uncontended fast path, adaptive spinning for micro-contention, and eventual
fairness. Its raw implementation uses an atomic byte and parks only on the
contended slow path. This directly matches a short critical section with a
small number of worker threads.

The relevant experiment must change only the mutex implementation. It should
retain Fastvid's scoped worker threads, relaxed atomic tile assignment,
`Vec<Option<T>>` output, tile granularity, and final ordered collection. This
avoids the repeated collection and worker-local allocations that confounded
the rejected Rayon variants.

The crate and its core dependencies are dual MIT/Apache-2.0 and therefore
compatible with Fastvid's intended MIT distribution. It is an implementation
detail with no stream-format or specification impact.

## Limits

The output lock is acquired once per tile, not per sample. With 256x128 tiles,
prediction and entropy work may completely amortize the standard mutex fast
path. A smaller lock or adaptive spin is not evidence of end-to-end benefit;
the balanced four-thread video matrix must show it. Scheduler counters are
useful only if wall time advances, because whole-process context-switch counts
can be dominated by benchmark and allocator behavior.

## Relevant experiments

- [EXP-0044: parking-lot output mutex](../experiments/EXP-0044-parking-lot-output-mutex.md)
