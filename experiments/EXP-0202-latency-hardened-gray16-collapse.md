# EXP-0202 — Latency-hardened gray16 collapse probe

Status: **REJECTED**

Date: 2026-07-30

Baseline revision: `e8fd199` (codec source retained from accepted EXP-0200).
Baseline codec-source SHA-256:
`a694cd12c51b445edb6f6e33e5f2b7f4a0611aa23d53e66301a59ee150d78b74`.
Evaluator revision: `ed4febd5b9b2815fc86e1f36e50ef4a63b8eac46`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

## Hypothesis and rationale

EXP-0201's encoder-only gray16 collapse selector was byte-for-byte identical
to accepted EXP-0200 on rejection and therefore retained its strict quality
and rate improvement, but its rebuilt candidate recorded a new 0.510400 ms
RGB10 decode median against the 0.5 ms gate. The cached accepted baseline was
0.480144 ms. The selector could not proceed to the full tier where its intended
natural-gray16 repair occurs.

The order-0 shard decoder constructs a 4096-entry shared lookup table before
four serial rANS lanes decode the payload. The accepted launch uses 128
threads. Raising only this entropy kernel to 256 threads gives twice as many
workers to the scalable table-construction phase without altering rANS state
order, reconstruction scheduling, pixels, syntax, or allocations. Wavefront
reconstruction remains at the accepted 128 threads; EXP-0193 established that
geometry independently.

The candidate combines this bounded decode-headroom correction with exactly
EXP-0201's encoder selector: in gray16's 0.10--0.20 baseline-byte/sample band,
retain step 321 only when its exact refined payload collapses below 0.10
byte/sample. The falsifiable hypothesis is that rejection clears the RGB10
latency gate while preserving accepted rate/quality, and unchanged full
restores the mid-rate natural gray16 samples, preserves procedural stability,
strictly improves the 85.083862 generation SSIM floor, reduces bytes, and adds
no failures.

## Canonical command and artifacts

```sh
PYTHONPATH=.:cuda python3 scripts/evaluate.py --tier <rejection|full> \
  --output <source-keyed-artifact> --libvship-revision v5.0.0 \
  --libvship-build '5.0.0 CUDA direct C API' \
  --libvship-gpu-id 0 --libvship-workers 2
```

- rejection baseline cache hit:
  `evaluation_results/rejection-a694cd12c51b445edb6f6e33e5f2b7f4a0611aa23d53e66301a59ee150d78b74.json`;
- full baseline cache hit if required:
  `evaluation_results/full-a694cd12c51b445edb6f6e33e5f2b7f4a0611aa23d53e66301a59ee150d78b74.json`.

Candidate codec-source SHA-256:
`dd884487bdba6b6ac52ec5f1fd717c2ed11bcbbeeff791a24cefb627c315606e`.

- candidate rejection:
  `evaluation_results/rejection-dd884487bdba6b6ac52ec5f1fd717c2ed11bcbbeeff791a24cefb627c315606e.json`.

## Result

The focused CUDA suite passed all 14 tests. Compression and quality on the
rejection subset remained byte-for-byte identical to EXP-0200 and EXP-0201:
322,199,847 bytes at 6.680316x, ordinary extrema 94.813339 / 0.747542,
generation extrema 89.327042 / 2.308000, and the same five quality failures.

The 256-thread order-0 launch regressed decode materially and introduced three
performance failures:

- `ai-01-rgb444-10`: 0.619920 ms;
- `performance-1080p-rgb444-10`: 0.617952 ms; and
- `ai-05-rgb444-16`: 0.593088 ms.

Candidate rejection artifact SHA-256 is
`22bd5488869a7547d1257a28252637f1c291cd1c3e266c490805156c3cb89a37`.

## Conclusion

Reject at rejection and restore accepted EXP-0200. Doubling block width for
the order-0 table build reduces effective occupancy enough to overwhelm any
additional table-fill parallelism; it is not a source of decoder headroom.
The collapse selector still has no valid full-tier artifact. A successor must
reduce actual order-0 work or memory traffic rather than increase its launch
width, and must not overwrite either failed source-keyed result.

Related: [EXP-0193](EXP-0193-latency-hardened-gray-repair.md),
[EXP-0200](EXP-0200-med-funded-stable-gray16.md), and
[EXP-0201](EXP-0201-gray16-refinement-collapse-probe.md).
