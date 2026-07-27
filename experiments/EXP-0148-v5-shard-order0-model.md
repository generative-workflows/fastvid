# EXP-0148 — v5 shard-local order-0 complete-byte model

Status: **ACCEPTED**

## Hypothesis

The EXP-0147 adaptive quality oracle reaches 14.352886x while preserving a
minimum luma XPSNR above 50 dB. Independently adding the established order-0
rANS candidate to each version-5 4,096-symbol entropy shard will reduce
complete stream bytes by at least 5%, enough for the adaptive rate-quality
oracle to exceed 15x without changing reconstruction.

## Modification

Add a read-only v5 shard analyzer. Decode each existing zero-run, four-lane
Rice, or fixed-block shard to folded residuals and charge the established
normalized order-0 model for its table, payload, final state, byte rounding,
and existing three-byte shard header. Retain the smaller current/candidate
size independently per shard and retain all stream header and directory bytes.
Do not change or materialize a bitstream.

## Test

- Verify analyzed shard symbols equal the frame sample count.
- Verify current shard bytes plus header/directory overhead equal the exact v5
  stream length.
- Require the fallback oracle never to exceed the current stream.
- Run q80, q85, and q90 over all 24 corpus-v3 codec samples and the 15-sample
  1080p slice.
- Join XPSNR and adaptive selections to the deterministic EXP-0147 table.

## Gate

- Accept the entropy family for exact implementation if the charged oracle
  saves at least 5% complete bytes and pushes the adaptive corpus result above
  15x at greater than 50 dB minimum luma XPSNR.
- Reject shard-local order-0 if it saves less than 3%.
- Between 3% and 5%, investigate plane/tile-shared tables before changing the
  format.

## References

- [Research 0024](../research/0024-finite-block-ans-entropy-models.md)
- [Research 0025](../research/0025-context-conditioned-residual-entropy.md)
- [EXP-0053](EXP-0053-finite-block-order0-model.md)
- [EXP-0055](EXP-0055-modeled-rans-selector.md)
- [EXP-0147](EXP-0147-v5-full-corpus-rate-distortion-sweep.md)

## Result

All 72 q80/q85/q90 cells completed over the 24-sample corpus-v3 first-frame
panel. Current encoded bytes exactly reproduced EXP-0147. The fully charged
per-shard fallback oracle measured:

| Quality/control | Current | Charged order-0 oracle | Complete-byte saving | Winning shards |
|---|---:|---:|---:|---:|
| q80 | 14.678455x | 17.911841x | 18.052% | 27,029/37,926 |
| q85 | 13.377732x | 15.994952x | 16.363% | 27,049/37,926 |
| q90 | 11.687517x | 13.678019x | 14.553% | 27,598/37,926 |
| EXP-0147 first-frame quality oracle | 14.352886x | 17.436014x | 17.683% | 27,245/37,926 |

The raw 72-row artifact SHA-256 is
`07dc6c01ecf802ec7b1bbbcd4b6db49f3dda24095850097c5e289e54db7aa65e`.
The public analyzer also verifies that analyzed samples equal frame samples
and current shard bytes plus unchanged header/directory bytes equal the exact
stream length.

During this experiment the quality target was tightened to minimum per-frame
XPSNR. EXP-0149 proved the first-frame quality oracle cannot qualify that
target. The entropy result remains a valid complete-byte screening result,
but its rate-quality row is not a codec target pass.

## Decision

Accept shard-local order-0 as the next exact entropy family: it clears the 5%
screening gate by a wide margin. Require full-frame corpus-v4 modeling and an
exact synchronized Rust/CUDA format implementation before claiming realized
compression or overall target compliance.
