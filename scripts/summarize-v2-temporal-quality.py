#!/usr/bin/env python3
import csv
import math
import sys
from collections import Counter, defaultdict
from pathlib import Path


if len(sys.argv) != 4:
    raise SystemExit(
        "usage: summarize-v2-temporal-quality.py INPUT.tsv OUTPUT.md OUTPUT.tsv"
    )

input_path, markdown_path, summary_path = map(Path, sys.argv[1:])
with input_path.open(newline="") as stream:
    rows = list(csv.DictReader(stream, delimiter="\t"))

keys = [(row["sample"], int(row["frame"]), int(row["quality"])) for row in rows]
if len(keys) != len(set(keys)):
    raise SystemExit("duplicate sample/frame/quality rows")
qualities = sorted({int(row["quality"]) for row in rows})
expected_frames = {row["sample"]: int(row["frames"]) for row in rows}
expected_total = sum(expected_frames.values())
for quality in qualities:
    quality_rows = [row for row in rows if int(row["quality"]) == quality]
    if len(quality_rows) != expected_total:
        raise SystemExit(
            f"q{quality} has {len(quality_rows)} rows, expected {expected_total}"
        )
if 100 in qualities:
    exact = [row for row in rows if int(row["quality"]) == 100]
    if not all(row["exact"] == "true" for row in exact):
        raise SystemExit("q100 is not exact for every frame")


def is_1080p(row):
    return (int(row["width"]), int(row["height"])) == (1920, 1080)


def is_4k(row):
    return (int(row["width"]), int(row["height"])) == (3840, 2160)


def summarize(selected, scope, quality):
    raw = sum(int(row["raw_bytes"]) for row in selected)
    encoded = sum(int(row["encoded_bytes"]) for row in selected)
    y_scores = [float(row["xpsnr_y_db"]) for row in selected]
    minimum_plane = min(float(row["xpsnr_frame_min_db"]) for row in selected)
    return {
        "scope": scope,
        "quality": quality,
        "frames": len(selected),
        "ratio": raw / encoded,
        "minimum_y_xpsnr": min(y_scores),
        "minimum_plane_xpsnr": minimum_plane,
        "passing_frames": sum(
            row["xpsnr_y_gt_50db"] == "true" for row in selected
        ),
        "exact_frames": sum(row["exact"] == "true" for row in selected),
        "keyframes": sum(row["keyframe"] == "true" for row in selected),
        "simultaneous_pass": raw / encoded > 15 and min(y_scores) > 50,
    }


by_quality = defaultdict(list)
by_sample = defaultdict(list)
for row in rows:
    by_quality[int(row["quality"])].append(row)
    by_sample[row["sample"]].append(row)

summaries = []
for quality in qualities:
    selected = by_quality[quality]
    summaries.extend(
        [
            summarize(selected, "corpus", str(quality)),
            summarize([row for row in selected if is_1080p(row)], "1080p", str(quality)),
            summarize([row for row in selected if is_4k(row)], "4k", str(quality)),
        ]
    )

sample_quality = {}
sample_adaptive = []
for sample, candidates in by_sample.items():
    candidates_by_quality = defaultdict(list)
    for row in candidates:
        candidates_by_quality[int(row["quality"])].append(row)
    eligible = [
        quality
        for quality, quality_rows in candidates_by_quality.items()
        if all(row["xpsnr_y_gt_50db"] == "true" for row in quality_rows)
    ]
    chosen = min(eligible)
    sample_quality[sample] = chosen
    sample_adaptive.extend(candidates_by_quality[chosen])
summaries.extend(
    [
        summarize(sample_adaptive, "corpus-sample-adaptive", "adaptive"),
        summarize(
            [row for row in sample_adaptive if is_1080p(row)],
            "1080p-sample-adaptive",
            "adaptive",
        ),
        summarize(
            [row for row in sample_adaptive if is_4k(row)],
            "4k-sample-adaptive",
            "adaptive",
        ),
    ]
)

fields = [
    "scope",
    "quality",
    "frames",
    "ratio",
    "minimum_y_xpsnr",
    "minimum_plane_xpsnr",
    "passing_frames",
    "exact_frames",
    "keyframes",
    "simultaneous_pass",
]
summary_path.parent.mkdir(parents=True, exist_ok=True)
with summary_path.open("w", newline="") as stream:
    writer = csv.DictWriter(stream, fields, delimiter="\t")
    writer.writeheader()
    for row in summaries:
        writer.writerow(
            {
                **row,
                "ratio": f"{row['ratio']:.9f}",
                "minimum_y_xpsnr": (
                    "inf"
                    if math.isinf(row["minimum_y_xpsnr"])
                    else f"{row['minimum_y_xpsnr']:.6f}"
                ),
                "minimum_plane_xpsnr": (
                    "inf"
                    if math.isinf(row["minimum_plane_xpsnr"])
                    else f"{row['minimum_plane_xpsnr']:.6f}"
                ),
                "simultaneous_pass": str(row["simultaneous_pass"]).lower(),
            }
        )

selection_counts = Counter(sample_quality.values())
lines = [
    "# Version-2 temporal rate-quality feasibility screen",
    "",
    f"All {expected_total} corpus-v4 frames are encoded with GOP 12 for sequences "
    "and GOP 1 for stills. Every frame is independently scored with XPSNR; "
    "complete per-frame stream headers and directories are charged.",
    "",
    "| Scope | Q | Frames | Keyframes | Compression | Min frame Y XPSNR | Min frame/plane XPSNR | >50 dB frames | Exact | >15x and >50 dB |",
    "|---|---:|---:|---:|---:|---:|---:|---:|---:|:---:|",
]
for row in summaries:
    minimum_y = (
        "inf"
        if math.isinf(row["minimum_y_xpsnr"])
        else f"{row['minimum_y_xpsnr']:.4f} dB"
    )
    minimum_plane = (
        "inf"
        if math.isinf(row["minimum_plane_xpsnr"])
        else f"{row['minimum_plane_xpsnr']:.4f} dB"
    )
    lines.append(
        f"| {row['scope']} | {row['quality']} | {row['frames']} | "
        f"{row['keyframes']} | {row['ratio']:.6f}x | {minimum_y} | "
        f"{minimum_plane} | {row['passing_frames']}/{row['frames']} | "
        f"{row['exact_frames']}/{row['frames']} | "
        f"{'yes' if row['simultaneous_pass'] else 'no'} |"
    )
lines += [
    "",
    "The sequence-consistent sample-adaptive control uses "
    + ", ".join(
        f"q{quality} for {count} samples"
        for quality, count in sorted(selection_counts.items())
    )
    + ".",
    "",
    "This is a feasibility control for temporal redundancy, not a proposal to "
    "promote the version-2 bitstream or its CPU-oriented entropy structure.",
    "",
]
markdown_path.parent.mkdir(parents=True, exist_ok=True)
markdown_path.write_text("\n".join(lines))
