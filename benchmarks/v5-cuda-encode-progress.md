# CUDA v5 encoder progress

Complete-call medians use three warm-ups and ten trials on the NVIDIA L40.
Input is the 3840x2160 Calotes 10-bit frame used by the decode baseline. Every
row was compared byte-for-byte with the Rust v5 stream. GP/s counts luma
pixels; host orchestration, compact size transfer, allocation, and every CUDA
kernel are included.

| Experiment | Q | Median | Encode | Raw | Ratio | Rust bytes |
|---|---:|---:|---:|---:|---:|:---:|
| EXP-0140 serial analysis | 90 | 60.366 ms | 0.137 GP/s | 0.550 GB/s | 11.227x | exact |
| EXP-0140 serial analysis | 100 | 60.480 ms | 0.137 GP/s | 0.549 GB/s | 5.177x | exact |
| EXP-0141 parallel analysis | 90 | 2.958 ms | 2.804 GP/s | 11.218 GB/s | 11.227x | exact |
| EXP-0141 parallel analysis | 100 | 2.720 ms | 3.050 GP/s | 12.198 GB/s | 5.177x | exact |
| EXP-0143 warp Rice emission | 90 | 2.197 ms | 3.776 GP/s | 15.104 GB/s | 11.227x | exact |
| EXP-0143 warp Rice emission | 100 | 2.201 ms | 3.769 GP/s | 15.077 GB/s | 5.177x | exact |
| EXP-0145 warp block packing | 90 | 2.228 ms | 3.723 GP/s | 14.894 GB/s | 11.227x | exact |

EXP-0140 q90 profiling attributed 57.842 ms (96.1% of CUDA time) to exact
entropy analysis, 1.036 ms to prediction, and 1.194 ms to emission. EXP-0141
reduced analysis to 389.410 us; prediction (1.039 ms) and emission (1.105 ms)
then became the dominant stages. EXP-0143 reduced q90 emission to 385.121 us
(65.1%) while prediction remained 1.040 ms and analysis measured 374.593 us.
The complete q90 call is now 25.9% above the >3 GP/s target on this 4K frame;
EXP-0145 additionally removes the fixed-block bottleneck exposed by the full
corpus while preserving 3.723 GP/s on this 4K control. The refreshed corpus
and 1080p results remain the scaling test.
