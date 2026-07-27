#!/usr/bin/env python3
import csv
import math
import sys
from collections import Counter, defaultdict
from pathlib import Path


if len(sys.argv) != 5:
    raise SystemExit(
        "usage: summarize-v5-entropy-model.py MODEL.tsv QUALITY.tsv OUTPUT.md OUTPUT.tsv"
    )

model_path, quality_path, markdown_path, summary_path = map(Path, sys.argv[1:])
with model_path.open(newline="") as stream:
    models = list(csv.DictReader(stream, delimiter="\t"))
with quality_path.open(newline="") as stream:
    quality_rows = list(csv.DictReader(stream, delimiter="\t"))

model_keys = [
    (row["sample"], int(row["frame"]), int(row["quality"])) for row in models
]
if len(model_keys) != len(set(model_keys)):
    raise SystemExit("duplicate sample/frame/quality model rows")

quality_by_key = {
    (row["sample"], int(row["frame"]), int(row["quality"])): row
    for row in quality_rows
}
qualities = sorted({int(row["quality"]) for row in models})
expected_frame_keys = {
    (row["sample"], int(row["frame"]))
    for row in quality_rows
    if int(row["quality"]) in qualities
}
for quality in qualities:
    actual = {
        (row["sample"], int(row["frame"]))
        for row in models
        if int(row["quality"]) == quality
    }
    if actual != expected_frame_keys:
        raise SystemExit(
            f"q{quality} model coverage differs from the quality artifact"
        )


def summarize(rows, scope, quality):
    raw = sum(int(row["raw_bytes"]) for row in rows)
    current = sum(int(row["encoded_bytes"]) for row in rows)
    oracle = sum(int(row["oracle_stream_bytes"]) for row in rows)
    shards = sum(int(row["shards"]) for row in rows)
    winners = sum(int(row["order0_winning_shards"]) for row in rows)
    xpsnr = [
        float(
            quality_by_key[
                (row["sample"], int(row["frame"]), int(row["quality"]))
            ]["xpsnr_y_db"]
        )
        for row in rows
    ]
    return {
        "scope": scope,
        "quality": quality,
        "frames": len(rows),
        "shards": shards,
        "winning_shards": winners,
        "current_ratio": raw / current,
        "oracle_ratio": raw / oracle,
        "saving_percent": 100.0 * (1.0 - oracle / current),
        "minimum_xpsnr": min(xpsnr),
        "simultaneous_pass": oracle > 0 and raw / oracle > 15 and min(xpsnr) > 50,
    }


groups = defaultdict(list)
for row in models:
    groups[int(row["quality"])].append(row)

summaries = []
for quality, rows in sorted(groups.items()):
    summaries.append(summarize(rows, "corpus", str(quality)))
    summaries.append(
        summarize(
            [row for row in rows if (int(row["width"]), int(row["height"])) == (1920, 1080)],
            "1080p",
            str(quality),
        )
    )
    summaries.append(
        summarize(
            [row for row in rows if (int(row["width"]), int(row["height"])) == (3840, 2160)],
            "4k",
            str(quality),
        )
    )

selected_quality = {}
for row in quality_rows:
    if row["xpsnr_y_gt_50db"] == "true":
        key = (row["sample"], int(row["frame"]))
        candidate = int(row["quality"])
        if key not in selected_quality or candidate < selected_quality[key]:
            selected_quality[key] = candidate
adaptive = [
    row
    for row in models
    if int(row["quality"])
    == selected_quality[(row["sample"], int(row["frame"]))]
]
summaries.append(summarize(adaptive, "corpus-frame-oracle", "adaptive"))
summaries.append(
    summarize(
        [row for row in adaptive if (int(row["width"]), int(row["height"])) == (1920, 1080)],
        "1080p-frame-oracle",
        "adaptive",
    )
)
summaries.append(
    summarize(
        [row for row in adaptive if (int(row["width"]), int(row["height"])) == (3840, 2160)],
        "4k-frame-oracle",
        "adaptive",
    )
)

fields = [
    "scope", "quality", "frames", "shards", "winning_shards", "current_ratio",
    "oracle_ratio", "saving_percent", "minimum_xpsnr", "simultaneous_pass",
]
summary_path.parent.mkdir(parents=True, exist_ok=True)
with summary_path.open("w", newline="") as stream:
    writer = csv.DictWriter(stream, fields, delimiter="\t")
    writer.writeheader()
    for row in summaries:
        writer.writerow({
            **row,
            "current_ratio": f"{row['current_ratio']:.9f}",
            "oracle_ratio": f"{row['oracle_ratio']:.9f}",
            "saving_percent": f"{row['saving_percent']:.6f}",
            "minimum_xpsnr": "inf" if math.isinf(row["minimum_xpsnr"]) else f"{row['minimum_xpsnr']:.6f}",
            "simultaneous_pass": str(row["simultaneous_pass"]).lower(),
        })

selection_counts = Counter(selected_quality.values())
lines = [
    "# Version-5 shard-local order-0 screening",
    "",
    "The oracle independently keeps the current zero-run/Rice/fixed-block body or substitutes a fully charged order-0 body in each 4,096-symbol shard. Order-0 bytes include normalized tables, final states, byte rounding, and the existing three-byte shard record header. Stream header and directory bytes are retained.",
    "",
    "| Scope | Q | Frames | Winning shards | Current compression | Charged oracle | Complete-byte saving | Min Y XPSNR | >15x and >50 dB |",
    "|---|---:|---:|---:|---:|---:|---:|---:|:---:|",
]
for row in summaries:
    minimum = "inf" if math.isinf(row["minimum_xpsnr"]) else f"{row['minimum_xpsnr']:.4f} dB"
    lines.append(
        f"| {row['scope']} | {row['quality']} | {row['frames']} | {row['winning_shards']}/{row['shards']} | "
        f"{row['current_ratio']:.6f}x | {row['oracle_ratio']:.6f}x | {row['saving_percent']:.3f}% | "
        f"{minimum} | {'yes' if row['simultaneous_pass'] else 'no'} |"
    )
lines += [
    "",
    "The optimistic per-frame quality control uses "
    + ", ".join(f"q{quality} for {count}" for quality, count in sorted(selection_counts.items()))
    + " frames, matching the full-frame quality audit.",
    "",
    "This is a screening bound, not a claimed format result. A passing candidate still requires exact payload materialization plus matched Rust/CUDA encode/decode measurements.",
    "",
]
markdown_path.parent.mkdir(parents=True, exist_ok=True)
markdown_path.write_text("\n".join(lines))
