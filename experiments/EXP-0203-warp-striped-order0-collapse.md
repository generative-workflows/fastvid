# EXP-0203 — Warp-striped order-0 table build with gray16 collapse probe

Status: **ACCEPTED**

Date: 2026-07-30

Baseline revision: `e59ee94` (codec source retained from accepted EXP-0200).
Baseline codec-source SHA-256:
`a694cd12c51b445edb6f6e33e5f2b7f4a0611aa23d53e66301a59ee150d78b74`.
Evaluator revision: `ed4febd5b9b2815fc86e1f36e50ef4a63b8eac46`.
Corpus revision: `fastvid-corpus-v1-extracted-2`.

## Hypothesis and rationale

EXP-0201's encoder-only gray16 collapse selector retained accepted rejection
rate/quality but recorded a marginal RGB10 decode failure. EXP-0202 attempted
headroom by doubling the order-0 block from 128 to 256 threads; three decode
latency samples regressed to 0.59--0.62 ms, showing that lower occupancy costs
more than additional block-wide table workers save.

The actual table-build imbalance remains: one CUDA thread currently writes all
lookup slots assigned to one rANS symbol. A dominant symbol can serialize
hundreds or thousands of stores while neighboring threads finish. The
candidate retains the accepted 128-thread block and maps one four-warp block
to four symbols at a time. Each warp stripes a symbol's `[cumulative,
cumulative+frequency)` range across its 32 lanes, then advances by four
symbols. Every slot receives the identical byte exactly once; table syntax,
rANS state order, payload reads, reconstruction, error validation, and shared
memory size are unchanged.

The encoder change is exactly EXP-0201's probe: within gray16's 0.10--0.20
baseline-byte/sample band, retain step 321 only when its exact refined payload
falls below 0.10 byte/sample. The falsifiable hypothesis is that warp-striped
construction clears rejection latency without changing bytes or pixels, then
full restores mid-rate natural gray16 while retaining the procedural repair,
strictly raises the 85.083862 generation SSIM floor, reduces bytes, introduces
no failures, and passes all performance gates.

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
`a99b654aa9736264a930fb44627d03f1c799ae48d55e5817a895147e8c865580`.

- candidate rejection:
  `evaluation_results/rejection-a99b654aa9736264a930fb44627d03f1c799ae48d55e5817a895147e8c865580.json`.

## Result

The focused CUDA suite passed all 14 tests. Rejection compression and quality
are byte-for-byte and metric-for-metric identical to accepted EXP-0200:
322,199,847 bytes at 6.680316x, ordinary extrema 94.813339 / 0.747542,
generation extrema 89.327042 / 2.308000, and the same five quality failures.

All performance gates pass. The controlling 1080p RGB10 encode/decode medians
are 0.939984/0.484400 ms versus the cached accepted baseline's
0.943392/0.480144 ms. Warp striping therefore avoids the failures seen in
EXP-0201 and EXP-0202 while keeping latency within the gate. Rejection artifact
SHA-256 is
`424d0f8633684937593d24b78af6adc7c63e7ced2ee8c4082661ff2b6699b2ee`.

The unchanged source advanced to full:

- candidate full:
  `evaluation_results/full-a99b654aa9736264a930fb44627d03f1c799ae48d55e5817a895147e8c865580.json`.

| Codec | Bytes | Ratio | Ordinary min/max | Generation min/max | Failures |
|---|---:|---:|---:|---:|---:|
| baseline | 2,122,552,061 | 6.447785x | 88.391052 / 1.632229 | 85.083862 / 4.645103 | 173 |
| candidate | 2,121,516,178 | 6.450934x | 88.391052 / 1.632229 | 85.946732 / 4.645103 | 173 |

The candidate saves 1,035,883 bytes (0.0488%), strictly raises the actual
full-corpus generation SSIMULACRA2 floor by 0.862869, introduces no new
failure identity, and passes every correctness, coverage, determinism, and
performance gate. Full artifact SHA-256 is
`e2ecb66e2ccf311dd139435e029df4aac69d571eb454735b2d0d5a92313f32c2`.

The selector has the predicted content behavior. `raw-47-gray-16` returns
from 1,859,438 to 1,593,948 bytes and improves generation quality from
85.083862 / 2.202852 to 89.148621 / 1.934746. `raw-02-gray-16` and
`game-supertuxkart-gray-16` also return to their smaller baseline-step streams.
Meanwhile `procedural-02-gray-16` remains continuously refined at 325,059
source bytes and 87.259956 / 3.196179 generation quality.

## Conclusion

Accept under the failing-baseline exception. Exact refined-payload collapse is
a better gray16 content classifier than baseline rate alone: it preserves the
synthetic stability win while avoiding natural-content drift and saves bytes.
Warp-striped order-0 table construction retains 128-thread occupancy, clears
all latency gates, and provides the valid unchanged full artifact that the
encoder-only EXP-0201 could not obtain. Retain both changes as the new pre-v1
baseline.

Related: [EXP-0201](EXP-0201-gray16-refinement-collapse-probe.md),
[EXP-0202](EXP-0202-latency-hardened-gray16-collapse.md), and
[research 0039](../research/0039-parallel-rice-bitstream-hardware.md).
