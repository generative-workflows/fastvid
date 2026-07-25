# EXP-0077 — High-bit prefix Rice streaming

Status: **ACCEPTED**

## Classification

**Speed-frontier exploitation** — combine EXP-0076's high-bit profile with
the causal/sampled Rice-selection findings in research 0027. This is applied
only to the isolated fixed-predictor branch from EXP-0074.

## Hypothesis

A deterministic middle-row source-neighbor proxy can identify dense
Rice-coded fixed-gradient tiles and estimate their parameter cheaply enough
to stream the real residuals directly. Falling back to exact buffered
selection whenever the proxy favors zero-run should:

- improve focused one-thread high-bit encoding by at least 10%;
- keep q90 bytes within 2% of the fixed EXP-0074 branch;
- preserve every reconstructed sample and quality metric exactly; and
- leave decoding unchanged.

The proxy may select a non-optimal Rice parameter, so byte equality is not
expected. It reuses existing version-2 syntax and changes no decoder.

## Modification

Starting with the EXP-0074 fixed high-bit patch:

1. On spatial tiles, inspect the middle source row.
2. Form a cheap clamp-gradient proxy from source left/above/upper-left
   neighbors, quantize it with the existing table, and calculate exact
   sample-row zero-run and Rice costs.
3. If the sampled winner is Rice, allocate one output payload and perform the
   real causal reconstructed-neighbor pass once, writing Rice codes directly
   with the sampled parameter.
4. If the sampled winner is zero-run, use EXP-0074's established exact
   folded-vector and `finish_entropy` path.
5. Keep direct temporal handling, 8-bit code, bitstream syntax, tile geometry,
   quality mapping, and decoder unchanged.

This differs from rejected EXP-0050 by avoiding 17 counters in the real
reconstruction loop. It differs from the universal estimator variants in
EXP-0063 by retaining exact fallback for sparse tiles.

## Fast test

Use the EXP-0074 six-trial focused harness on the checksummed native-10-bit
motion sequence at q90/q100, one/four threads:

- compare against the preserved EXP-0074 fixed branch, not the exhaustive
  production source;
- require deterministic bytes and metrics within each cell;
- report entropy-mode counts and Rice parameters;
- compare q90 one-thread throughput with OpenAPV `fastest` at 80.724 MP/s.

Advance only if the focused gate passes. Then run q90/q100 on the complete
native high-bit supplement and native high-bit access.

## Gate

- one-thread q90 encode improves at least 10%;
- q90 bytes increase no more than 2%;
- all reconstruction metrics and maximum error exactly match EXP-0074;
- q100 remains exact;
- decode remains within 5%;
- no malformed-stream or independent-tile regression.

Beating OpenAPV remains the project target. A candidate that advances but
does not exceed 80.724 MP/s must retain a quantified remaining gap and proceed
to a separate SIMD or format experiment.

## Result

The six-trial focused matrix passed every gate:

| Quality | Threads | Variant | Bytes | Encode | Decode |
|---:|---:|---|---:|---:|---:|
| 90 | 1 | fixed EXP-0074 | 18,882,860 | 49.425 MP/s | 62.626 MP/s |
| 90 | 1 | prefix streaming | 18,882,860 | 67.620 MP/s | 64.990 MP/s |
| 90 | 4 | fixed EXP-0074 | 18,882,860 | 146.298 MP/s | 152.998 MP/s |
| 90 | 4 | prefix streaming | 18,882,860 | 190.234 MP/s | 158.149 MP/s |
| 100 | 1 | fixed EXP-0074 | 32,239,138 | 52.139 MP/s | 60.350 MP/s |
| 100 | 1 | prefix streaming | 32,246,235 | 65.281 MP/s | 62.151 MP/s |
| 100 | 4 | fixed EXP-0074 | 32,239,138 | 144.685 MP/s | 144.013 MP/s |
| 100 | 4 | prefix streaming | 32,246,235 | 169.660 MP/s | 141.168 MP/s |

At q90, one-thread encoding improved **36.81%** and four-thread encoding
improved **30.03%** with a byte-identical stream and identical
52.001930 dB Y-PSNR, 0.99373056 SSIM, and maximum error 4. q100 one-thread
improved 25.21%, bytes increased 0.022%, and reconstruction remained exact.
The first q90 frame retained the fixed branch's 15 zero-run, 15 Rice-0, and
15 Rice-4 tiles.

The complete native high-bit confirmation measured:

| Quality | Threads | Encode geomean | Decode geomean | Total bytes |
|---:|---:|---:|---:|---:|
| 90 | 1 | +16.25% | +2.41% | +0.000% |
| 90 | 4 | +8.36% | -0.33% | +0.000% |
| 100 | 1 | +19.94% | +0.40% | +0.623% |
| 100 | 4 | +15.63% | -2.12% | +0.623% |

The result is content-dependent but remains a valid frontier branch. At q90
one thread, 10-bit gradient and motion improved 35.91% and 37.03%;
12-bit UI was flat (+0.03%); and 16-bit motion regressed 1.97%. At q100 all
four samples improved 14.61% to 24.61%; the largest byte increase was 1.917%
on 16-bit motion, within the declared 2% bound.

The 32-cell native-video access comparison measured 1.09% lower geometric
mean latency overall (1.64% lower at q90 and 0.54% lower at q100). Dependency
frames and decoded-frame counts were unchanged. Per-cell timing ranged from
-7.45% to +7.32%, consistent with short access-job VM noise; bytes-read
geomean rose 0.172% because q100 keyframes can use the sampled Rice parameter.

On the matched q90 diagnostic, OpenAPV `fastest` remains 19.38% faster
relative to the candidate (80.724 versus 67.620 MP/s), or the candidate is
16.23% below the target. This is a substantial reduction from EXP-0073's
4.88x gap but does not yet satisfy the project goal.

Correctness controls for q100 high-bit round-trip, q90 high-bit error bounds,
and independent version-2 mode/tile decode passed. The full library suite
passed 52/54 tests. As in EXP-0074, the two failures assert that production
encoding uses the exhaustive predictor oracle or legacy Paeth; they are
policy mismatches for an isolated speed branch, not stream or reconstruction
failures. Strict Clippy and formatting passed after removing a needless
source-level `return`; measured timings use the semantically identical
pre-cleanup binary retained below.

Artifacts:

- focused matrix:
  `artifacts/exp0077-prefix-rice-focused.tsv`
  (`8c1f4e2043da7804d04364771cf7a038b7d122995c598f7e7791539392ea11ff`);
- complete native high-bit matrix:
  `artifacts/exp0077-prefix-rice-highbit.tsv`
  (`4722f13c2b605b204141baae808dc851eac1bcdc80e0d031d6ba93ddbef97a3e`);
- native high-bit access matrix:
  `artifacts/exp0077-prefix-rice-access.tsv`
  (`77858aa6f167a29c484c6389ab9fec15d40c4d3acb424685c53a0e94ec8406f0`);
- exact measured source patch:
  `artifacts/frontier/exp0077-highbit-stream-measured.patch`
  (`72210c221f2340c4ea19e8832c4b28b2fde39b9dae52e394bbe7936145885c47`);
- measured candidate binary:
  `artifacts/frontier/fastvid-highbit-stream-exp0077`
  (`74cc27bb2a45e2df79fbb0ff7be7fbd274dcb1858b3181560a1bd895d8dfe9d6`).

## Decision

Accept as a non-dominated high-bit speed technology branch and restore the
maximum-compression working source. The estimator eliminates most profiled
entropy-finalization work on Rice tiles while retaining exact sparse-tile
fallback, byte-identical q90 streams across the current supplement, and
unchanged decoding.

Do not yet replace the three-slot speed binary: first combine this high-bit
branch with the established 8-bit fixed-gradient speed source, rerun the
automatic internal frontier, and refresh the separate matched OpenAPV panel.
The remaining 16.23% matched throughput deficit should be attacked with a
profiled SIMD/direct-emission kernel, not more predictor or transform search.

## References

- [Research 0027](../research/0027-streaming-rice-parameter-selection.md)
- [Research 0019](../research/0019-modern-integer-entropy-kernels.md)
- [EXP-0050](EXP-0050-high-bit-streaming-selector-cost.md)
- [EXP-0063](EXP-0063-sampled-streaming-rice.md)
- [EXP-0074](EXP-0074-fixed-predictor-high-bit-speed.md)
- [EXP-0076](EXP-0076-fixed-high-bit-perf-profile.md)
