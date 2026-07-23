# Research 0010 — Single-frame random-access evaluation

Status: **REVIEWED**

## Sources

- J. Bankoski et al., [RFC 6386: VP8 Data Format and Decoding
  Guide](https://www.rfc-editor.org/rfc/rfc6386), Sections 3 and 4.
- A. Filippov et al., [RFC 8761: Video Codec Requirements and Evaluation
  Methodology](https://www.rfc-editor.org/rfc/rfc8761), Sections 4.2.2 and 5.
- Alliance for Open Media, [AV2 Common Test Conditions
  v9](https://aomedia.org/docs/CWG-G082_AV2_CTC_v9.pdf), all-intra and random
  access configurations.

## Findings

- A keyframe is a random-access point, not a promise that every later frame is
  directly accessible. Reconstructing a predicted target requires decoding
  the dependency chain from the nearest preceding keyframe.
- Common test conditions keep all-intra and random-access configurations
  separate because compression, structural delay, and access cost differ.
- Sequential decode throughput alone hides editor behavior. Scrubbing,
  thumbnails, cuts, and reverse traversal repeatedly request isolated frames,
  making target latency and dependency amplification first-class metrics.
- Container lookup and storage I/O are separate from codec reconstruction.
  An in-memory benchmark should measure codec-only access first, then a future
  indexed sequence container can add lookup and I/O measurements.

## Fastvid implications

For each standard video and quality, benchmark isolated target offsets 0, 1,
`GOP/2`, and `GOP-1` in every GOP. Start at the nearest preceding keyframe,
decode through the target, and discard preroll output. Record:

- target frame and keyframe indices;
- dependency/preroll frames and total frames decoded;
- compressed bytes read from keyframe through target;
- target access latency;
- useful-target MP/s and actual-work MP/s;
- access amplification (`decoded_frames / 1 requested frame`).

Report median, p95, and worst latency across targets. Compare all-intra GOP 1
against short-GOP 12 without mixing their compression results. The initial
harness is warm-cache and excludes source/container I/O.

## Relevant experiments

- [EXP-0005](../experiments/EXP-0005-gated-temporal-prediction.md) established
  the GOP-12 coding baseline.
- [EXP-0009](../experiments/EXP-0009-single-frame-access.md) adds the access
  benchmark and standard reporting protocol.
