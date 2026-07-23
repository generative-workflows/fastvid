# 0006 — Standard image/video codec evaluation methodology

Sources:

- A. Filippov et al., “Video Codec Requirements and Evaluation Methodology,”
  IETF RFC 8761, 2020: https://www.rfc-editor.org/rfc/rfc8761.html
- Alliance for Open Media, “AV2 Common Test Conditions v9.0,” 2026:
  https://aomedia.org/docs/CWG-G082_AV2_CTC_v9.pdf
- Xiph.Org, “Video Codec Testing and Quality Measurement”:
  https://netvc.github.io/testing/
- Xiph.Org lossless Blender Open Movie masters:
  https://media.xiph.org/

Terms: RFC code components are Simplified BSD. AOM methodology documents and
the Xiph testing tooling are openly accessible reference material. The chosen
core media is separately licensed CC BY 3.0 (*Big Buck Bunny*) and CC BY 2.5
(*Elephants Dream*) and remains outside Git.

## Findings

RFC 8761 requires multiple operating points and per-plane PSNR plus luma
MS-SSIM; this supports curves rather than conclusions from one quality value.
It also separates objective measurements from subjective evaluation.

The AV2 common test conditions define explicit content classes, frame counts,
coding configurations, thread/tile settings, quality points, tool versions,
and memory reporting. Its recent revisions retain separate all-intra,
random-access, and low-delay configurations. Reproducibility requires keeping
those configurations distinct.

Xiph's methodology uses lossless sources, standard still and video sets, and
short subsets for frequent tests. Xiph's media archive provides lossless
individual 1080p PNG frames for permissively licensed Blender Open Movies,
making small verified excerpts practical without downloading entire films.

## Fastvid implications

1. Maintain a small mandatory core and a broader periodic corpus.
2. Version frame selections, conversion parameters, licenses, and hashes.
3. Test five quality points and report each color plane.
4. Separate all-intra and short-GOP results.
5. Time only codec work, use warmups and medians, and preserve per-sample
   outliers.
6. Add MS-SSIM, VMAF, memory, and BD-rate as the harness matures.
7. Do not admit non-commercial or purpose-restricted media to the standard
   corpus.

## Relevant experiments

- [EXP-0003](../experiments/EXP-0003-regression-corpus.md) implements the
  initial versioned core corpus and rate-distortion harness.
- Future short-GOP experiments will use the video track defined here.
