#!/usr/bin/env python3
import csv
import math
import sys
from collections import Counter, defaultdict
from pathlib import Path


if len(sys.argv) != 5:
    raise SystemExit(
        "usage: summarize-v5-frame-quality.py INPUT.tsv FIRST_FRAME.tsv OUTPUT.md OUTPUT.tsv"
    )

input_path, first_frame_path, markdown_path, summary_path = map(Path, sys.argv[1:])
with input_path.open(newline="") as stream:
    rows = list(csv.DictReader(stream, delimiter="\t"))
with first_frame_path.open(newline="") as stream:
    first_frame_rows = list(csv.DictReader(stream, delimiter="\t"))

keys = [(row["sample"], int(row["frame"]), int(row["quality"])) for row in rows]
if len(keys) != len(set(keys)):
    raise SystemExit("duplicate sample/frame/quality rows")

qualities = sorted({int(row["quality"]) for row in rows})
expected_frames = {
    row["sample"]: int(row["frames"])
    for row in rows
}
expected_total = sum(expected_frames.values())
for quality in qualities:
    quality_rows = [row for row in rows if int(row["quality"]) == quality]
    if len(quality_rows) != expected_total:
        raise SystemExit(
            f"q{quality} has {len(quality_rows)} rows, expected {expected_total}"
        )

first_frame_control = {
    (row["sample"], int(row["quality"])): row
    for row in first_frame_rows
    if int(row["quality"]) in qualities
}
for row in rows:
    if int(row["frame"]) != 0:
        continue
    key = (row["sample"], int(row["quality"]))
    control = first_frame_control.get(key)
    if control is None:
        continue
    if row["encoded_bytes"] != control["encoded_bytes"]:
        raise SystemExit(f"first-frame byte mismatch for {key}")
    if row["xpsnr_y_db"] != control["xpsnr_y_db"]:
        raise SystemExit(f"first-frame XPSNR mismatch for {key}")


def is_hd(row):
    return (int(row["width"]), int(row["height"])) == (1920, 1080)


def is_4k(row):
    return (int(row["width"]), int(row["height"])) == (3840, 2160)


def summarize(selected, scope, quality):
    raw = sum(int(row["raw_bytes"]) for row in selected)
    encoded = sum(int(row["encoded_bytes"]) for row in selected)
    y_scores = [float(row["xpsnr_y_db"]) for row in selected]
    plane_minimum = min(float(row["xpsnr_frame_min_db"]) for row in selected)
    return {
        "scope": scope,
        "quality": quality,
        "frames": len(selected),
        "ratio": raw / encoded,
        "minimum_y_xpsnr": min(y_scores),
        "minimum_plane_xpsnr": plane_minimum,
        "passing_frames": sum(
            row["xpsnr_y_gt_50db"] == "true" for row in selected
        ),
        "exact_frames": sum(row["exact"] == "true" for row in selected),
        "simultaneous_pass": raw / encoded > 15 and min(y_scores) > 50,
    }


by_quality = defaultdict(list)
by_frame = defaultdict(list)
by_sample = defaultdict(list)
for row in rows:
    by_quality[int(row["quality"])].append(row)
    by_frame[(row["sample"], int(row["frame"]))].append(row)
    by_sample[row["sample"]].append(row)

summaries = []
for quality in qualities:
    summaries.append(summarize(by_quality[quality], "corpus", str(quality)))
    summaries.append(
        summarize(
            [row for row in by_quality[quality] if is_hd(row)],
            "1080p",
            str(quality),
        )
    )
    summaries.append(
        summarize(
            [row for row in by_quality[quality] if is_4k(row)],
            "4k",
            str(quality),
        )
    )

frame_adaptive = []
for candidates in by_frame.values():
    eligible = [
        row for row in candidates if row["xpsnr_y_gt_50db"] == "true"
    ]
    frame_adaptive.append(min(eligible, key=lambda row: int(row["quality"])))

sample_adaptive = []
sample_quality = {}
for sample, candidates in by_sample.items():
    candidates_by_quality = defaultdict(list)
    for row in candidates:
        candidates_by_quality[int(row["quality"])].append(row)
    eligible_qualities = [
        quality
        for quality, quality_rows in candidates_by_quality.items()
        if all(row["xpsnr_y_gt_50db"] == "true" for row in quality_rows)
    ]
    chosen = min(eligible_qualities)
    sample_quality[sample] = chosen
    sample_adaptive.extend(candidates_by_quality[chosen])

summaries.extend(
    [
        summarize(sample_adaptive, "corpus-sample-adaptive", "adaptive"),
        summarize(
            [row for row in sample_adaptive if is_hd(row)],
            "1080p-sample-adaptive",
            "adaptive",
        ),
        summarize(
            [row for row in sample_adaptive if is_4k(row)],
            "4k-sample-adaptive",
            "adaptive",
        ),
        summarize(frame_adaptive, "corpus-frame-oracle", "adaptive"),
        summarize(
            [row for row in frame_adaptive if is_hd(row)],
            "1080p-frame-oracle",
            "adaptive",
        ),
        summarize(
            [row for row in frame_adaptive if is_4k(row)],
            "4k-frame-oracle",
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

worst = {}
for quality, quality_rows in by_quality.items():
    worst[quality] = min(quality_rows, key=lambda row: float(row["xpsnr_y_db"]))

sample_counts = Counter(sample_quality.values())
frame_counts = Counter(int(row["quality"]) for row in frame_adaptive)
lines = [
    "# Version-5 full-frame rate-quality audit",
    "",
    f"Every one of the {expected_total} corpus frames is scored independently. "
    "The headline quality value is the minimum frame-level luma XPSNR; "
    "sequence averages are not used for the gate.",
    "",
    "| Scope | Q | Frames | Compression | Min frame Y XPSNR | Min frame/plane XPSNR | >50 dB frames | Exact | >15x and >50 dB |",
    "|---|---:|---:|---:|---:|---:|---:|---:|:---:|",
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
        f"{row['ratio']:.6f}x | {minimum_y} | {minimum_plane} | "
        f"{row['passing_frames']}/{row['frames']} | "
        f"{row['exact_frames']}/{row['frames']} | "
        f"{'yes' if row['simultaneous_pass'] else 'no'} |"
    )
lines += ["", "Worst fixed-quality frames:", ""]
for quality in qualities:
    row = worst[quality]
    lines.append(
        f"- q{quality}: `{row['sample']}` frame {row['frame']}, "
        f"{float(row['xpsnr_y_db']):.4f} dB luma XPSNR."
    )
lines += [
    "",
    "The per-sample adaptive control chooses one quality for every frame in a sample: "
    + ", ".join(
        f"q{quality} for {count} samples"
        for quality, count in sorted(sample_counts.items())
    )
    + ".",
    "",
    "The optimistic per-frame oracle chooses "
    + ", ".join(
        f"q{quality} for {count} frames"
        for quality, count in sorted(frame_counts.items())
    )
    + ". It is a decision bound, not an implemented rate-control result.",
    "",
    "All available first-frame encoded-byte and XPSNR controls match EXP-0147 exactly.",
    "",
]
markdown_path.parent.mkdir(parents=True, exist_ok=True)
markdown_path.write_text("\n".join(lines))
