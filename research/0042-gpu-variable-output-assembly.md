# GPU variable-length output assembly

## Sources and use constraints

- NVIDIA,
  [CUB / CUDA Core Compute Libraries](https://nvidia.github.io/cccl/unstable/cub/index.html)
  and
  [`DeviceSegmentedScan`](https://nvidia.github.io/cccl/unstable/cub/api/structcub_1_1DeviceSegmentedScan.html),
  current documentation and open-source implementation. CCCL is distributed
  under the Apache License 2.0 with LLVM exceptions, compatible with
  Fastvid's implementation constraints.
- NVIDIA,
  [CUDA cooperative-group scan](https://docs.nvidia.com/cuda/cuda-programming-guide/05-appendices/device-callable-apis.html#cooperative-groups-scan-h),
  current programming guide. This is API/algorithm documentation; no
  implementation is copied.
- Tian et al.,
  [*GPU Lossy Compression for HPC Can Be Versatile and
  Ultra-Fast*](https://doi.org/10.1145/3712285.3759817), SC 2025. The paper
  describes independent block compression followed by a global prefix sum of
  compressed block lengths. Fastvid uses the decomposition as evidence, not
  its lossy numeric transforms or source.
- NVIDIA,
  [nvCOMP](https://github.com/NVIDIA/nvcomp), an Apache-2.0 repository of
  documentation and examples for batched GPU compression. Compression source
  has not been published since nvCOMP 2.3, so Fastvid must not infer or copy
  unavailable implementation details.

## Common decomposition

Variable-length parallel output is a size-and-placement problem, not a reason
to serialize encoding:

```text
classify/count independent units
              ↓
exclusive sum of exact byte counts
              ↓
write each unit to output[offset .. offset + count]
```

Integer addition is associative and exact, so an exclusive scan of byte
counts produces deterministic canonical offsets independent of the scan
tree. CUB exposes device-wide, block-wide, warp-wide, and segmented scans;
its two-phase device API also makes temporary storage an explicit reusable
resource. Cooperative groups expose scan within a fixed thread group.

The 2025 VGC pipeline independently compresses blocks, globally prefix-sums
their compressed lengths, and then places them in the final stream. This is
recent independent evidence that the decomposition remains useful in
high-throughput GPU compressors. Its rate/quality results are not comparable
to Fastvid because its transforms, data, and fidelity target differ.

## Mapping Fastvid version 5

Version 5 already has the required exact local counts:

- one independent access-tile directory entry;
- 4,096 folded symbols per entropy shard;
- a three-byte shard record header (`mode`, `u16 body_length`);
- exact zero-run byte count;
- exact four-lane Rice byte counts, including three `u32` lane lengths;
- exact 128-symbol fixed-block byte count; and
- a selected body bounded below 65,536 bytes.

Flatten shards in canonical plane/tile/raster order. For selected shard record
size `s[i] = 3 + body_bytes[i]`, one global exclusive integer scan gives
`r[i]`, the shard offset relative to the payload region. A segmented or
per-tile reduction gives:

```text
tile_offset = payload_start + r[first_shard]
tile_length = sum(s[first_shard .. last_shard])
```

The last scan value plus the last size gives the complete allocation. Each
emission block then writes its record directly to its disjoint interval.
There is no output mutex and no final concatenation kernel. A fixed-stride
directory kernel writes tile metadata from the first-shard offsets and
per-tile sums.

The initial CUDA implementation should keep counts and offsets in 64-bit
storage even though a shard body is `u16` and a directory tile length is
`u32`. This matches the CPU encoder's checked-`usize` accumulation and avoids
overflow before final representability checks.

## Predictor handoff

Entropy output is not the only serial span. Full-tile clamp-gradient depends
on reconstructed left, above, and upper-left samples. A raster thread has
32,768 dependent steps for a full 256x128 tile. The same dependency graph has
`width + height - 1 = 383` antidiagonals; samples within one antidiagonal are
independent once the preceding diagonals are complete.

The initial GPU staging kernel should therefore assign one thread block per
tile and advance antidiagonals with block synchronization, writing folded
residuals to canonical raster positions. This changes execution order but
not the recurrence or stream. It is an explicit prototype choice, not yet
performance evidence: 383 barriers may outweigh the added parallelism on
small edge/chroma tiles. A competing kernel should assign independent scalar
tile chains to lanes or warps, following EXP-0129's successful CPU
interleaving. Both feed the same entropy-shard interface.

## Required GPU measurements

Report separately:

- host-to-device input transfer;
- predictor/reconstruction kernel;
- entropy candidate counting and selection;
- scan/reduction;
- selected-body emission;
- directory/header write;
- device-to-host stream transfer; and
- end-to-end encode/decode.

Also record predictor steps, maximum selected codeword/lane bytes, active
warps, achieved memory bandwidth, branch/warp efficiency, temporary and peak
device memory, and output bytes. CUDA Graph capture and buffer reuse should
be evaluated only after the first correct staged implementation establishes
launch and allocation shares.

## Relevant experiments

- [EXP-0100](../experiments/EXP-0100-parallel-serialization-budget.md)
- [EXP-0105](../experiments/EXP-0105-predictor-wavefront-model.md)
- [EXP-0110](../experiments/EXP-0110-full-tile-bounded-shards.md)
- [EXP-0120](../experiments/EXP-0120-direct-rice-lane-emission.md)
- [EXP-0129](../experiments/EXP-0129-interleaved-full-tile-predictors.md)
- [EXP-0134](../experiments/EXP-0134-cuda-handoff-contract.md)
