# Rayon work-stealing for tile scheduling

## Sources

- Rayon 1.12 [crate documentation][rayon-docs].
- Rayon [`ThreadPool` documentation][pool-docs].
- Rayon [`ThreadPoolBuilder` documentation][builder-docs].
- Rayon [`ParallelIterator` documentation][iterator-docs].
- Rayon [source repository and license][rayon-repo].

[rayon-docs]: https://docs.rs/rayon/1.12.0/rayon/
[pool-docs]: https://docs.rs/rayon/1.12.0/rayon/struct.ThreadPool.html
[builder-docs]: https://docs.rs/rayon/1.12.0/rayon/struct.ThreadPoolBuilder.html
[iterator-docs]: https://docs.rs/rayon/1.12.0/rayon/iter/trait.ParallelIterator.html
[rayon-repo]: https://github.com/rayon-rs/rayon

## Applicability

Fastvid's current `parallel_map` duplicates a small scheduler in the 8-bit and
high-bit codecs. Every encode or decode call:

1. spawns `threads` scoped operating-system threads;
2. assigns every tile through one atomic counter;
3. stores every completed tile through one central `Mutex`;
4. destroys all workers after the call.

This is especially costly for video because the benchmark and public API
encode/decode one frame per call. Tiles vary in cost by plane, entropy mode,
prediction, and edge dimensions, so dynamic balancing remains useful.

Rayon's indexed parallel iterators split random-access ranges into work that
can be stolen, while indexed collection preserves logical item order. A
user-created `ThreadPool` fixes the maximum worker count and `install` runs
parallel iterators in that pool. The documented global pool is unsuitable for
Fastvid's explicit per-call `threads` setting because it has one process-wide
size; cached private pools keyed by effective worker count preserve that
control.

The pool cache should be bounded by the host's available parallelism. Requests
above the physical/logical CPU availability should not create persistent
oversubscribed pools. The single-thread path should remain a normal iterator
with no Rayon dispatch. Pool construction belongs outside timed steady-state
work; the standard warm-up initializes it before recorded trials.

Rayon 1.12 requires a sufficiently recent Rust compiler (Fastvid currently
uses Rust 1.97.1) and is dual MIT/Apache-2.0 licensed. It is therefore
compatible with the project's intended MIT distribution. This is an
implementation dependency only and does not affect the bitstream or normative
codec specification.

## Risks and measurements

- Parallel collection may allocate temporary producer/consumer state even
  though it removes Fastvid's `Vec<Option<T>>` and per-tile output mutex.
- Multiple cached pool sizes retain worker stacks for the process lifetime.
  Capping effective sizes at `available_parallelism()` bounds this to at most
  the useful host sizes encountered.
- Work-stealing overhead can lose on a single still or very cheap tiles.
  Results must therefore separate still/video and encode/decode.
- Pool reuse is most relevant across video frames. A one-frame-only result
  cannot establish the claimed mechanism.

Measure thread=4 video first with balanced binaries, then stills and high-bit
content. Preserve one-thread controls, byte-identical streams, encoded sizes,
and quality signatures. For an accepted result, profile thread creation,
mutex/futex activity, cycles, and context switches where supported.

## Relevant experiments

- [EXP-0042: cached Rayon tile scheduler](../experiments/EXP-0042-cached-rayon-tile-scheduler.md)
- [EXP-0043: pooled worker-local tile scheduler](../experiments/EXP-0043-pooled-worker-local-tile-scheduler.md)
