# EXP-0168 — Stratified full corpus

Status: **ACCEPTED**

Date: 2026-07-29

## Hypothesis

Testing all 120 sources at all eight format/depth cells repeats nearly identical
content at multiple quantization depths. Assigning one depth per source per
color format should preserve content diversity and broad depth coverage while
substantially reducing full-tier runtime and extracted storage.

## Methodology

Use the frozen source-catalog order as the deterministic stratum index. Every
source receives:

- one YUV422 depth, rotating through 8, 10, and 16 bit;
- one RGB444 depth, alternating between 10 and 16 bit;
- one gray depth, rotating through 8, 10, and 16 bit with a one-position offset
  from YUV422.

Across 120 sources, the base assignment contains 360 quality cases: 40 examples
at each YUV422 depth, 60 at each RGB444 depth, and 40 at each gray depth. Union
in every frozen rejection case plus YUV422-10 and RGB444-10 for the first 24 4K
performance sources. This produces 394 unique quality cases plus the three
existing performance samples. Every source remains represented in every color
format, and every required matrix cell retains at least 40 independent sources.

Fresh extraction materializes only combinations referenced by the stratum or a
fixed rejection/performance case. The conversion math and raw plane layout are
unchanged. The extracted-corpus revision advances from
`fastvid-corpus-v1-extracted-1` to `fastvid-corpus-v1-extracted-2`.

## Static validation

The focused suite requires exact base counts, one case per format per source,
complete required-matrix coverage, stable assignment, and preservation of all
fixed rejection and performance cases.

```sh
PYTHONPATH=. pytest -q tests/test_extract_corpus.py tests/test_evaluate.py
```

Result: 17 passed.

## Size result

| Extraction | Quality combinations | Raw bytes | GiB |
|---|---:|---:|---:|
| Cartesian source × matrix | 960 | 26,127,360,000 | 24.33 |
| Stratified + required union | 394 | 11,682,662,400 | 10.88 |

A clean extraction is 55.3% smaller. Updating an existing extraction does not
delete obsolete raw files; they become unreferenced by the new manifest and can
be removed by rebuilding the corpus directory when storage reclamation is
needed.

## Canonical validation

Regeneration produced 120 sources and 397 manifest samples with SHA-256
`60395b46746037a573e2dd03e0876a12bb80ab93269dcaf2eb0e495088225d8f`.
The unchanged rejection tier passed all 11 samples with minimum SSIMULACRA2
`93.69731903076172`, maximum Butteraugli `0.08440515398979187`, and compression
ratio `6.188000859134071`.

The first practical full-tier run completed all 397 samples in 114.476 seconds.
It failed quality on 21 samples: 18 YUV422-8 cases and one each of RGB444-16,
gray-10, and gray-16. Minimum SSIMULACRA2 was `73.36583709716797`; maximum
Butteraugli was `0.392439603805542`. No correctness, coverage, compression, or
performance gate failed. These format/depth/source combinations were present in
the former Cartesian full tier, so this is a codec-baseline failure exposed by
the smaller runnable tier, not a score change caused by stratification.

Artifacts:

- `/tmp/fastvid-v7-stratified-corpus-rejection.json` (pass);
- `/tmp/fastvid-v7-stratified-corpus-full.json` (quality fail).

## Decision

Accept the stratified full corpus. It preserves all 120 sources, all three color
formats per source, at least 40 examples in every required matrix cell, and the
unchanged rejection/performance selections while cutting the nominal full
quality workload by 59.0%. The current codec is not an accepted baseline because
the stratified full run fails 21 per-frame quality gates. Do not interpret an untested depth of one particular
source as a proxy result for that source; matrix coverage is established across
the frozen stratum as a whole.
