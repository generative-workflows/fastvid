# Fastvid codec frontier

This is a current-state registry, not an experiment log or changelog. It keeps
two or three distinct codec versions that best represent the measured
technology frontier.
Completed evidence remains in immutable experiment records. The
machine-readable companion is [`frontier.json`](frontier.json).

## Frontier slots

| Slot | Source | Binary SHA-256 | Stream compatibility | Evidence | State |
|---|---|---|---|---|---|
| Practical compression | `4ad0318` | `1235c7e82cf34fdddf5c341a5c17d265687368092174d175db709f22b17131c9` | v2 encode; v0/v1/v2 decode | [EXP-0052](experiments/EXP-0052-16bit-temporal-decode-guard.md) | Preserved |
| Maximum compression | `36b1d20` | `d4d7edaf68a67601f753652757d62bcc49ff237e9ef0954ad0174ddc45322a14` | 8-bit v3 / high-bit v2 encode; legacy decode | [EXP-0068](experiments/EXP-0068-four-state-rans.md) | Preserved |
| Speed | `5cb2508` + `exp0078-unified-speed.patch` | `bf1002e7e790bb5607180ff2874edd57957536c83cce620982f0a6999614ccb3` | 8-bit v3 / high-bit v2 encode; legacy decode | [EXP-0078](experiments/EXP-0078-unified-speed-frontier.md) | Preserved |

The speed source is reproduced by applying
`artifacts/frontier/exp0078-unified-speed.patch` to Git commit `5cb2508`; the
other two active base sources are retained directly in Git. The speed tier
uses fixed clamp-gradient intra prediction and frame-gated temporal
prediction at every bit depth. Its high-bit path samples one source row to
stream Rice tiles directly and retains exact buffered fallback for sparse
zero-run tiles. It trades some spatial compression for materially higher
encode, decode, and single-frame-access throughput.

The maximum-compression tier retains scalar order-0 rANS on small payloads
and uses four interleaved states only when their 12-byte cost is at most 0.5%
of the modeled scalar payload. This keeps tiny UI/cut streams byte-identical
while exposing instruction-level parallelism on larger camera and graphics
tiles.

The former balanced snapshot (`156054c`, binary `06ef3278…6ab8`) remains a
reproducible historical reference under EXP-0045. EXP-0061 retired it from
routine active measurement because speed gains 26.07% encode and 41.23%
decode throughput for a 7.93% compression-ratio cost.

## Automated view

The current comparable fast-feedback view is
[`benchmarks/frontier.svg`](benchmarks/frontier.svg), with exact aggregates in
[`benchmarks/frontier-summary.tsv`](benchmarks/frontier-summary.tsv).
`scripts/benchmark-frontier.sh` validates every binary hash and records a
balanced multi-version run; `scripts/graph-frontier.py` validates that matrix,
finds non-dominated points, and regenerates both files. This is a screening
view, not a substitute for complete corpus, access, quality, or memory
evidence.

## Matched OpenAPV external reference

OpenAPV is not an internal Fastvid slot and is not plotted on the incompatible
four-case 8-bit aggregate. The separate
[`matched 10-bit graph`](benchmarks/openapv-frontier.svg) compares all three
preserved Fastvid binaries with pinned OpenAPV v0.3.0.0 on the same
checksummed 1280x720, 24-frame, 10-bit YUV 4:2:2 sequence. Both codecs are
all-intra with 256x128 tiles and measured at one/four threads.

The one-thread q90-neighborhood rows are:

| Codec | Control | Ratio | Encode MP/s | Decode MP/s | Y PSNR |
|---|---:|---:|---:|---:|---:|
| Fastvid speed | q90 | 4.685392x | 67.301 | 64.511 | 52.001930 |
| Fastvid practical | q90 | 5.307903x | 16.642 | 59.517 | 52.002293 |
| Fastvid maximum | q90 | 5.307903x | 16.802 | 59.620 | 52.002293 |
| OpenAPV medium | QP 22 | 4.408004x | 17.666 | 62.658 | 51.534665 |
| OpenAPV fastest | QP 23 | 4.464067x | 80.431 | 61.956 | 51.735588 |

OpenAPV controls are the measured rows nearest practical Fastvid q90 Y-PSNR:
the remaining gaps are -0.468 dB and -0.267 dB. The Fastvid speed point is a
separate rate/speed tradeoff: versus `fastest`, it uses 4.72% less bitrate at
0.266 dB higher Y-PSNR, encodes 16.32% more slowly (OpenAPV is 19.51%
faster), and decodes 4.12% faster. At four threads OpenAPV encodes 16.42%
faster and Fastvid decodes 4.68% faster.

At the high-fidelity boundary, Fastvid speed q100 is exact at 2.743688x,
64.077 MP/s encode, and 257.969880 Mb/s. OpenAPV `fastest` QP0 is not exact
(`max_error=2`) at 1.965097x, 62.481 MP/s, and 360.180000 Mb/s. Fastvid is
2.55% faster to encode and uses 28.38% less bitrate at this distinct,
higher-fidelity boundary; it is not called a nominal q100 match. Complete
one/four-thread rows are
in [`benchmarks/openapv-frontier-summary.tsv`](benchmarks/openapv-frontier-summary.tsv);
provenance and controls are in
[EXP-0078](experiments/EXP-0078-unified-speed-frontier.md).

## Active technology tree

```text
                              accepted balanced line
                                      |
                    rolling reconstruction / exact Rice
                       /                              \
       compatible predictor oracle              fixed-gradient speed tier
                |                              direct intra/temporal paths
        version-2 tile modes
          /             \
 practical guard     maximum compression
 16-bit temporal     budgeted four-state rANS
```

## Promotion and retirement

A candidate enters a slot only after its immutable experiment record contains:

- a source commit or source archive hash;
- a distinct release binary SHA-256;
- exact-stream controls and quality evidence;
- complete encoded bytes, encode/decode throughput, and access behavior for
  the candidate's declared scope; and
- the artifact hashes needed to reproduce its position.

At identical input, quality, coding track, and thread count, version A
dominates B only when A is no worse outside the measurement tolerance in
encoded bytes, quality, encode speed, decode speed, and access cost, and is
materially better in at least one. The standard tolerances are exact quality
invariance at q100, 1% for encoded bytes, and 5% for timing. A deliberate
rate/quality tradeoff remains non-dominated when it occupies a declared slot.

When a new version dominates a slot, this file replaces the row; the old
experiment remains immutable and Git retains its source. At most three
versions are active so confirmation cost remains bounded.

## Preserved artifacts

- `artifacts/frontier/fastvid-compression-exp0052`
  (`1235c7e82cf34fdddf5c341a5c17d265687368092174d175db709f22b17131c9`);
- `artifacts/frontier/fastvid-rans4-exp0068`
  (`d4d7edaf68a67601f753652757d62bcc49ff237e9ef0954ad0174ddc45322a14`);
- `artifacts/frontier/fastvid-speed-exp0078`
  (`bf1002e7e790bb5607180ff2874edd57957536c83cce620982f0a6999614ccb3`),
  reproduced by `artifacts/frontier/exp0078-unified-speed.patch`.

Historical reference:

- `artifacts/frontier/fastvid-speed-exp0060`
  (`f8e6bb69d7cf52b4531210e7423ec75a5626549ac1bacc964c1e123ca2bde8f7`);
- `artifacts/frontier/fastvid-rans-exp0055`
  (`dda826459cfa9cb017b751749d2b780419b18cc1a2ff9ff309492ea8b4df61da`);
- `artifacts/frontier/fastvid-balanced-exp0045`
  (`06ef3278e9055f3c53c94cf964f4a7bf785453b696e0df262dec9161b45c6ab8`).
