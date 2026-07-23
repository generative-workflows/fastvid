# EXP-0011 — Remove per-tile result-lock contention

Status: **REJECTED**

## Hypothesis

Replacing the atomic work queue and shared result mutex in `parallel_map` with
deterministic contiguous worker ranges will improve four-thread throughput and
memory locality without changing encoded bytes or decoded pixels.

## Modification

Pending measurement. The proposed implementation gives each scoped worker a
contiguous range of tile indices, returns a private vector, and concatenates
worker results in range order. It removes:

- one atomic fetch per tile;
- one mutex acquisition per completed tile;
- random worker assignment of adjacent tiles.

Thread creation remains unchanged so its cost can be isolated in a later
experiment.

## Test

1. Record two fast-tier baselines from EXP-0010.
2. Implement contiguous range partitioning.
3. Record two candidate fast-tier runs.
4. Require bit-identical encoded output and exact decoded equality in tests.
5. Confirm an accepted candidate on the standard corpus at q90/q100,
   one/four threads, GOP 1.

## Acceptance criteria

- Four-thread fast-tier encode or decode aggregate improves by at least 3%
  without a greater than 2% regression in the other direction.
- One-thread aggregate does not regress by more than 2%.
- Compression and reconstruction are bit-identical.
- Full-corpus confirmation agrees with the fast-tier direction.

## Results

Contiguous ranges removed the atomic counter and result mutex, but assigned
equal tile counts rather than equal work. On the most relevant four-thread
temporal case, median throughput changed:

| Metric | Baseline range | Contiguous range |
|---|---:|---:|
| Encode | 135.954–141.510 MP/s | 91.962–92.502 MP/s |
| Decode | 187.868–209.880 MP/s | 131.994–139.706 MP/s |

Using the closest run endpoints, the candidate regressed encode by 34.6% and
decode by 33.4%. Camera and scene-cut one-thread cases were effectively
unchanged, as expected. Unit tests passed and encoded byte counts were
unchanged.

The 360p feedback case also revealed excessive timing noise and was replaced
by a 4K case before EXP-0010 was accepted. This does not affect the decisive
four-thread temporal result above.

## Conclusion

Rejected. Contiguous ownership creates severe load imbalance because tile cost
depends on plane dimensions and entropy content. Preserve dynamic scheduling;
a follow-up may remove only the result mutex.


## References

- [Research 0011](../research/0011-openapv.md)
- [Research 0012](../research/0012-simd-cache-profiling.md)
- [EXP-0010](EXP-0010-fast-feedback-loop.md)
