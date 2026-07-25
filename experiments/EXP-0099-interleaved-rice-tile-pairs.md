# EXP-0099 — Interleaved Rice tile pairs

Status: **ACCEPTED**

## Classification

**Architecture exploration** — expose scalar instruction-level parallelism
across two format-independent causal tiles.

## Hypothesis

Alternating predictor/reconstruction work from two same-mode Rice tiles lets
the out-of-order CPU overlap two otherwise serial quantizer-load chains.
Because matched q90 has 30 Rice-0 luma tiles and Rice writing remains 28.80%
of encode-only samples, pairing should improve one-thread matched encode by
at least 3% over EXP-0095 without changing any tile bytes.

## Modification

1. In the all-intra one-thread path, group directory-adjacent tiles in pairs
   while retaining original output order.
2. Sample both selectors once.
3. When both tiles share plane, dimensions, and Rice parameter 0 or 4,
   alternate their reconstruction work within four-sample groups and keep
   separate exact writer states.
4. Use the accepted single-tile path for every other pair or odd tail.
5. Dispatch the prototype only for one-thread, all-intra, lossy 10-bit
   encoding. Leave exact, 12/16-bit, multi-thread, and temporal paths on
   their established kernels after the broader prototype showed that they
   did not have enough eligible pairs to repay its dispatch overhead.

No syntax, predictor, quantizer, entropy decision, tile geometry, decoder, or
quality mapping changes.

## Gate

- paired outputs exactly equal two independent scalar tile outputs for
  Rice-0/Rice-4, odd dimensions, and lossy reconstruction;
- focused q90/q100 streams and metrics are byte-identical;
- at least 3% matched q90 one-thread encode improvement;
- decode no worse than 5%;
- strict Clippy, formatting, and relevant release tests pass; and
- no multi-thread or slow-tier implementation unless the one-thread gate
  passes.

## Result

The targeted release test passes for Rice parameters 0 and 4 over two
odd-width, lossy tiles. A complete encoded frame from the matched
high-precision-motion sequence is byte-identical between EXP-0095 and this
candidate:

```text
c5ea50d34693a45626671ef056c6500b5afd39d688ac42cd035f83900972cf80
```

The initial two-trial smoke measured +23.942% 10-bit encode, then an
eight-trial confirmation measured +24.867%. After narrowing dispatch to
the proven regime, the balanced six-trial q90/q100 confirmation measured:

| Quality | Depth | Encode delta | Decode delta |
|---:|---:|---:|---:|
| 90 | 10 | +23.741% | +0.895% |
| 90 | 16 | +1.150% | +1.900% |
| 100 | 10 | -0.266% | +1.747% |
| 100 | 16 | -0.639% | +0.677% |

All encoded byte counts, PSNRs, block SSIMs, and maximum errors are
identical. On the complete native high-bit supplement at q90/one thread,
10-bit encode improves +19.985% geometrically, comprising +14.847% on the
HDR gradient and +25.352% on high-precision motion. Decode is -0.484%.
The non-dispatched 12/16-bit rows vary by amounts consistent with run noise
and retain identical outputs.

Validation:

- `cargo fmt --check`: pass;
- strict release Clippy: pass;
- targeted release exactness test: pass;
- full release suite: 53 pass / 5 fail, with the same five pre-existing
  selector-policy failures as EXP-0095.

Artifacts:

- candidate binary:
  `artifacts/frontier/fastvid-speed-exp0099-tile-pairs`
  (`41f5719eb0630cc8dd78067806dfe4775b30d9e3b9b59e0701775d40c91e71af`);
- smoke:
  `artifacts/exp0099-tile-pairs-smoke.tsv`
  (`6c3a5027f9f525275da655c697f72a5dca07688801f5310ae4e62b22d42bc6d2`);
- broad-prototype confirmation:
  `artifacts/exp0099-tile-pairs-confirm.tsv`
  (`10ef823388d03b08cb3a80a46887fb67edca3b12c2de952f37dd7df8b6e0f79e`);
- q90/q100 broad-prototype run:
  `artifacts/exp0099-tile-pairs-q90-q100.tsv`
  (`0e0342cf02357e1d9f5574242d9ff2c39a631fabb155361a2e0ab5bec9a997b3`);
- final focused confirmation:
  `artifacts/exp0099-tile-pairs-final.tsv`
  (`5b9629f06ab0cc82daf3b80cb4e7099e8ea399839b3a031bf2b8a0174aa26c45`);
- complete native confirmation:
  `artifacts/exp0099-tile-pairs-native.tsv`
  (`84905cdf91a5f9219949afc4c0ea73a5270a940950483a3a0ee063835a9c7bdd`).

## Decision

Accept the one-thread lossy 10-bit kernel. It clears the 3% gate by a wide
margin, preserves the format exactly, and demonstrates that independent
causal chains can improve CPU utilization without increasing codec
serialization. Keep the scope guard: this experiment does not justify a
quality-specific format decision or a claim that adjacent-pair scheduling
helps modes that were not selected on the standard corpus.

## References

- [Research 0036](../research/0036-independent-chain-software-pipelining.md)
- [EXP-0084](EXP-0084-specialized-rice-batching.md)
- [EXP-0095](EXP-0095-block-pack-rice4-combination.md)
- [EXP-0097](EXP-0097-post-rice4-profile.md)
