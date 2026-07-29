# EXP-0164 — FFVShip 5.0 Vulkan backend

Status: **REJECTED**

Date: 2026-07-29

## Hypothesis

FFVShip 5.0's experimental Vulkan backend may use the L40S more effectively
than its CUDA backend while preserving metric scores.

## Build and environment

Build the `v5.0.0` tag in an isolated temporary checkout using its checked-in
SPIR-V shaders and `make buildVulkan`. Link a separate CLI against FFmpeg 7 and
FFMS2. Do not replace `/usr/local/bin/FFVship`, which remains the validated CUDA
build. Installed build dependencies were `libvulkan-dev`, `vulkan-tools`, and
`mesa-vulkan-drivers` (about 128 MiB installed).

The container's injected NVIDIA 570.195.03 ICD is unusable. Both `vulkaninfo`
and the isolated FFVShip binary fail during instance creation:

```text
loader_scanned_icd_add: Could not get 'vkCreateInstance' via
'vk_icdGetInstanceProcAddr' for ICD libGLX_nvidia.so.0
VK_ERROR_INCOMPATIBLE_DRIVER
```

CUDA remains functional. The environment reports `NVIDIA_VISIBLE_DEVICES=void`
even though NVIDIA compute devices are mounted, indicating that Vulkan/graphics
device exposure differs from working CUDA access. Mesa llvmpipe was explicitly
selected with `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json` to test
the compiled Vulkan code path. FFVShip identifies it as Vulkan 5.0.0-a on
`llvmpipe (LLVM 20.1.2, 256 bits)`.

## Correctness control

Compare four deterministic 512x512, 10-bit YUV444 FFV1 frames against themselves
and against a deterministic noisy version. Use GPU id 0, three metric workers,
and two decoder threads for both backends.

Identical Butteraugli was zero for CUDA and Vulkan. SSIMULACRA2 was not 100 on
either backend and differed by about 0.28. On distorted inputs:

| Metric | CUDA range/mean | Vulkan range/mean | Difference |
|---|---:|---:|---:|
| SSIMULACRA2 | mean 92.7031 | mean 93.0659 | +0.3628 |
| Butteraugli max norm | mean 3.2822 | mean 3.0500 | -7.1% |

The backend therefore changes canonical scores materially. Results from CUDA
and Vulkan cannot share baselines or acceptance thresholds without a separate
metric-validation decision.

## Bounded 4K throughput probe

A full rejection run was intentionally not used after score interchangeability
failed. Instead, compare four deterministic noisy 3840x2160, 10-bit YUV444 FFV1
frames. Each metric runs separately, so these numbers include indexing, decode,
and process startup.

| Backend | SSIMULACRA2 wall | Butteraugli wall | SSIM CPU time | Butter CPU time |
|---|---:|---:|---:|---:|
| CUDA L40S | 12.96 s | 13.03 s | 18.65 s | 18.82 s |
| Vulkan llvmpipe | 13.51 s | 20.27 s | 62.45 s | 166.57 s |

At 4K, first-frame scores were much closer than the 512px control but still not
bit-identical: SSIMULACRA2 93.8976 versus 93.9393; Butteraugli maximum norm
2.90605 versus 2.90624. Software Vulkan used 3.3x the CPU time for SSIMULACRA2
and 8.9x for Butteraugli, with Butteraugli wall time 56% slower.

Transient 4K probe files were removed after measurement. Small control artifacts
remain under the directory named in `/tmp/fastvid-ffvship-control-path`; the
isolated source/build directory is named in
`/tmp/fastvid-vship-vulkan-build-path`.

## Decision

Reject Vulkan as a canonical backend on this machine. The NVIDIA Vulkan device
is unavailable inside the current container, llvmpipe is slower, and Vulkan
scores are backend-dependent. A future NVIDIA Vulkan test requires launching a
container where `vulkaninfo --summary` enumerates the L40S before FFVShip is run.
HIP is also compiled as a separate backend but is not applicable to this NVIDIA
host. FFVShip 5.0 exposes no separate native CPU backend; llvmpipe is the only
software path tested here.
