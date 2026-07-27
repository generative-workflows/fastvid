# XPSNR perceptual quality metric

## Sources and implementation

Christian Helmrich et al.,
[*XPSNR: A Low-Complexity Extension of the Perceptually Weighted Peak
Signal-To-Noise Ratio for High-Resolution Video Quality
Assessment*](https://doi.org/10.1109/ICASSP40776.2020.9054089), ICASSP 2020.

The authors describe XPSNR as a blockwise perceptually weighted extension of
PSNR intended to improve correlation with human judgments while retaining
low computational complexity. It is a full-reference metric and produces
logarithmic decibel values in a range comparable to PSNR. The paper reports
performance competitive with more complex MS-SSIM/VMAF-family measures on
its tested subjective datasets; that does not make the metrics
interchangeable.

Fastvid uses the
[FFmpeg `xpsnr` filter](https://ffmpeg.org/ffmpeg-filters.html#xpsnr), whose
implementation is authored by Fraunhofer HHI contributors and supports
planar YUV 4:2:2 at the relevant high bit depths. The benchmark records the
complete FFmpeg version/configuration, pixel format, frame size, frame rate,
per-plane sequence averages, and minimum plane average. Source and decoded
raw videos are passed without rescaling, color conversion, or chroma
resampling.

FFmpeg is GPL/LGPL software used as an external evaluation tool; it is not
linked into or distributed as part of the MIT Fastvid codec.

## Interpretation and guardrails

XPSNR is an additional focused/release axis:

- retain native-code-value PSNR, maximum error, and exact q100 equality;
- retain the specified full luma block-SSIM score;
- report Y, U, V, and minimum-plane XPSNR separately;
- preserve per-sample values before aggregating;
- never tune or accept a codec solely against XPSNR;
- do not label XPSNR as VMAF, MS-SSIM, DISTS, or ColorVideoVDP.

The current native high-bit corpus is procedural. These measurements are a
repeatable regression/GPU-handoff baseline, not evidence of subjective
quality on natural HDR production content.

## Relevant experiments

- [EXP-0135](../experiments/EXP-0135-cpu-gpu-baseline.md)
