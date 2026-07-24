# Finite-block ANS models for tile residuals

## Question

Fastvid's current entropy modes use one Rice parameter or canonical zero-run
tokens for an entire tile. Both are table-free and cheap, but neither adapts
to an arbitrary residual histogram. This review asks whether a table-driven
order-0 coder has enough *complete-byte* headroom to justify a format and
kernel experiment without weakening independent tile access.

## Open implementation sources

- Duda, [*Asymmetric numeral systems: entropy coding combining speed of
  Huffman coding with compression rate of arithmetic coding*][duda], 2014
  preprint.
- Giesen, [*Interleaved Entropy Coders*][giesen], 2014 preprint.
- Collet, [FiniteStateEntropy][fse], BSD-2-Clause implementation.
- Meta, [Zstandard compression format][zstd-format], including its normative
  FSE stream description.
- Kontoyiannis and Verdú, [*Lossless Data Compression at Finite
  Blocklengths*][finite], 2013, open preprint.
- Chen et al., [*A Review of the Asymmetric Numeral System and Its
  Applications to Digital Images*][review], 2022, CC BY 4.0.

[duda]: https://arxiv.org/abs/1311.2540
[giesen]: https://arxiv.org/abs/1402.3392
[fse]: https://github.com/Cyan4973/FiniteStateEntropy
[zstd-format]: https://github.com/facebook/zstd/blob/dev/doc/zstd_compression_format.md
[finite]: https://arxiv.org/abs/1212.2668
[review]: https://doi.org/10.3390/e24030375

The FSE repository identifies its implementation as BSD-2-Clause and
describes FSE as approaching arithmetic-coder density while retaining
table-driven speed. Giesen shows that multiple ANS states can be interleaved
without per-symbol metadata, exposing instruction-level parallelism and SIMD.
These sources make an isolated model and independent safe-Rust implementation
plausible. They do not by themselves establish that every ANS variant or
optimization is free of third-party patent claims; normative format work
still requires a focused IP review.

## Finite-block implications

The asymptotic entropy rate is not an adequate tile decision. Fastvid's full
tiles contain at most 32,768 samples, chroma and edge tiles are smaller, and
each tile must remain independently decodable. A practical order-0 mode must
charge:

- a normalized frequency table;
- the final coder state;
- byte rounding and padding;
- entropy-mode signaling;
- any alphabet remapping or escape symbols; and
- the cost of obtaining a table for an independently requested tile.

Kontoyiannis and Verdú show why finite-block behavior depends on information
variance as well as entropy. Collet's published FSE benchmark uses 32 KiB
blocks—the same scale as a full Fastvid luma tile—but synthetic probabilities
and an external benchmark cannot substitute for residual histograms and
Fastvid's table overhead.

## Model before implementation

The first experiment should recover the exact folded residual sequence from
each current stream and report three nested bounds:

1. empirical order-0 entropy, rounded to complete bytes, with no table cost;
2. a deterministic power-of-two normalized-frequency payload cost;
3. that payload plus a completely specified sparse table and final state.

The proposed conservative table representation stores a table-log byte,
varint symbol count, sorted symbol deltas, and normalized counts (the final
count is implied by the table total). It is intentionally simpler to specify
and charge than Zstandard's compressed normalized-count syntax. If even the
entropy lower bound has little headroom, ANS is rejected immediately. If only
the lower bound wins, table modeling—not an entropy kernel—is the next
problem.

Two locality choices matter:

- **tile-local table:** preserves the current strongest random-access and
  parallelism properties but repeats tables;
- **frame/plane table:** amortizes overhead and still permits independent tile
  payload decoding once a small frame header is available, but changes the
  container dependency and may fit individual tiles poorly.

The initial experiment models tile-local tables. A shared-table experiment is
justified only if the ideal entropy gap is large while table repetition erases
it.

## Performance implications

ANS does not automatically solve current speed problems. Table construction
adds encoder work, and a single state is serial. Giesen's interleaving result
is relevant only after complete bytes pass the gate: two or four independent
states may expose superscalar or SIMD execution without additional symbol
metadata. Fastvid should not implement that kernel before proving a rate
opportunity.

## Relevant experiments

- [EXP-0038: byte-oriented residual format
  model](../experiments/EXP-0038-byte-oriented-residual-model.md)
- [EXP-0053: finite-block order-0 entropy
  model](../experiments/EXP-0053-finite-block-order0-model.md)

