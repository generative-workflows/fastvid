# EXP-0105 — Predictor wavefront model

Status: **ACCEPTED**

## Classification

**Parallel-kernel exploration** — zero-rate topological execution of the
existing causal predictor.

## Hypothesis

Clamp-gradient samples on the same `x + y` anti-diagonal are independent.
A CUDA block can therefore reconstruct a full access tile in at most 383
barrier-separated rounds without changing the format. Splitting at the
accepted 64-row model should reduce maximum shared source storage from 64 KiB
to 32 KiB for high-bit luma and increase work-unit count, at the already
measured +0.756% native rate cost.

## Modification

Add a static model over exact corpus tile shapes. For full 128-row tiles and
64-row bands it reports:

- independently schedulable work units;
- maximum samples and anti-diagonal rounds per unit;
- maximum simultaneously active lanes;
- 32-lane warp-slot utilization; and
- maximum bytes if the source unit is staged in shared memory.

This is not a CUDA timing claim. The host has neither `nvcc` nor
`nvidia-smi`, so runtime occupancy and synchronization cost cannot be
measured here.

## Gate

- full tiles require no more than 383 rounds and 128 active lanes;
- 64-row bands require no more than 319 rounds, 64 active lanes, or 32 KiB
  staged high-bit source;
- both mappings remain within the 1,024-thread CUDA block limit; and
- the model is deterministic and checksummed.

## Result

Across exact corpus tile shapes:

| Mapping | Max samples | Max rounds | Max active lanes | Max staged 8-bit source | Max staged high-bit source |
|---|---:|---:|---:|---:|---:|
| 128-row access tile | 32,768 | 383 | 128 | 32 KiB | 64 KiB |
| 64-row band | 16,384 | 319 | 64 | 16 KiB | 32 KiB |

Full-tile 32-lane warp-slot utilization ranges from 81.97% to 84.40%;
64-row utilization ranges from 78.79% to 84.20%. At 1080p, full tiles expose
216 work units while 64-row splitting exposes 408. At 720p the counts are
90/180, and at 4K they are 765/1,530.

Every gate passes. Both mappings use far fewer than 1,024 active threads per
block. The output is deterministic:
`artifacts/exp0105-predictor-wavefront.tsv`
(`eff9baedc7fb4dcabd7948e9e8d68f4e723dc45a2229197451c252c7c629c6ae`).
Python bytecode compilation passes.

## Decision

Accept wavefront reconstruction as the preferred zero-rate predictor
parallelization branch. It reduces the dependency-DAG span from a scalar
32,768-sample traversal to 383 rounds without predictor restart syntax or
compression loss.

Keep 64-row bands as a fallback occupancy tradeoff: they nearly double work
units and halve staged source memory for the +0.756% native rate cost measured
in EXP-0104. Do not choose between them without CUDA measurements. The full
tile may be limited by 64-KiB staging and barrier cost; the band mapping may
be limited by only 64 useful lanes and boundary overhead.

The next combined design should pair full-tile diagonal reconstruction with
EXP-0102's four-lane Rice shards. It must model residual ordering and memory
coalescing before syntax: raster-order entropy output leads to strided
anti-diagonal reads, while diagonal-order Rice preserves Rice bit count but
can change zero-run and fixed-block grouping.

## References

- [Research 0037](../research/0037-parallel-hardware-friendly-codecs.md)
- [EXP-0104](EXP-0104-predictor-band-height-ladder.md)
