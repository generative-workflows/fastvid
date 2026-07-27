#!/usr/bin/env python3
"""Summarize the joint CPU-encode/CUDA-decode feedback artifacts."""

import csv
import math
import statistics
import sys
from collections import defaultdict
from pathlib import Path


if len(sys.argv) != 4:
    raise SystemExit("usage: summarize-cuda-feedback.py PREFIX OUTPUT.md OUTPUT.tsv")

prefix = Path(sys.argv[1])
markdown_path = Path(sys.argv[2])
tsv_path = Path(sys.argv[3])


def read_rows(suffix):
    with Path(f"{prefix}-{suffix}.tsv").open(newline="") as stream:
        return list(csv.DictReader(stream, delimiter="\t"))


def geometric_mean(values):
    return math.exp(statistics.fmean(math.log(value) for value in values))


encode_trials = read_rows("encode")
quality_rows = read_rows("quality")
decode_rows = read_rows("decode")
summary = []
resolution_summary = []

encode_cells = defaultdict(list)
for row in encode_trials:
    encode_cells[(row["sample"], int(row["quality"]), int(row["threads"]))].append(
        float(row["encode_mpps"]) / 1000.0
    )
encode_groups = defaultdict(list)
for (sample, quality, threads), values in encode_cells.items():
    encode_groups[(quality, str(threads))].append(statistics.median(values))
for (quality, threads), values in sorted(encode_groups.items()):
    summary.append({
        "axis": "rust_encode",
        "quality": quality,
        "setting": f"{threads} threads",
        "samples": len(values),
        "geometric_gpps": geometric_mean(values),
        "minimum_gpps": min(values),
        "target_gpps": 3.0,
        "speed_pass_samples": sum(value > 3.0 for value in values),
    })

decode_groups = defaultdict(list)
for row in decode_rows:
    decode_groups[(int(row["quality"]), row["placement"])].append(float(row["decode_gpps"]))
for (quality, placement), values in sorted(decode_groups.items()):
    summary.append({
        "axis": "cuda_decode",
        "quality": quality,
        "setting": placement,
        "samples": len(values),
        "geometric_gpps": geometric_mean(values),
        "minimum_gpps": min(values),
        "target_gpps": 5.0,
        "speed_pass_samples": sum(value > 5.0 for value in values),
    })

quality_groups = defaultdict(list)
for row in quality_rows:
    quality_groups[int(row["quality"])].append(row)
for quality, rows in sorted(quality_groups.items()):
    xpsnr = [float(row["xpsnr_y_db"]) for row in rows]
    summary.append({
        "axis": "quality_rate",
        "quality": quality,
        "setting": "corpus",
        "samples": len(rows),
        "total_ratio": sum(int(row["raw_bytes"]) for row in rows) / sum(int(row["encoded_bytes"]) for row in rows),
        "ratio_pass_samples": sum(row["compression_gt_15x"] == "true" for row in rows),
        "minimum_xpsnr_db": min(xpsnr),
        "xpsnr_pass_samples": sum(row["xpsnr_gt_50db"] == "true" for row in rows),
        "exact_samples": sum(row["exact"] == "true" for row in rows),
    })

dimensions = {row["sample"]: (int(row["width"]), int(row["height"])) for row in decode_rows}
hd_samples = {sample for sample, size in dimensions.items() if size == (1920, 1080)}
for threads in (1, 4):
    values = [
        statistics.median(cell_values)
        for (sample, quality, cell_threads), cell_values in encode_cells.items()
        if sample in hd_samples and quality == 90 and cell_threads == threads
    ]
    resolution_summary.append({
        "axis": "rust_encode_1080p", "quality": 90, "setting": f"{threads} threads",
        "samples": len(values), "geometric_gpps": geometric_mean(values),
        "minimum_gpps": min(values), "target_gpps": 3.0,
        "speed_pass_samples": sum(value > 3.0 for value in values),
    })
for placement in ("dram", "vram"):
    values = [
        float(row["decode_gpps"]) for row in decode_rows
        if row["sample"] in hd_samples and int(row["quality"]) == 90 and row["placement"] == placement
    ]
    resolution_summary.append({
        "axis": "cuda_decode_1080p", "quality": 90, "setting": placement,
        "samples": len(values), "geometric_gpps": geometric_mean(values),
        "minimum_gpps": min(values), "target_gpps": 5.0,
        "speed_pass_samples": sum(value > 5.0 for value in values),
    })
hd_quality = [row for row in quality_rows if row["sample"] in hd_samples and int(row["quality"]) == 90]
resolution_summary.append({
    "axis": "quality_rate_1080p", "quality": 90, "setting": "1920x1080",
    "samples": len(hd_quality),
    "total_ratio": sum(int(row["raw_bytes"]) for row in hd_quality) / sum(int(row["encoded_bytes"]) for row in hd_quality),
    "ratio_pass_samples": sum(row["compression_gt_15x"] == "true" for row in hd_quality),
    "minimum_xpsnr_db": min(float(row["xpsnr_y_db"]) for row in hd_quality),
    "xpsnr_pass_samples": sum(row["xpsnr_gt_50db"] == "true" for row in hd_quality),
    "exact_samples": sum(row["exact"] == "true" for row in hd_quality),
})

fields = [
    "axis", "quality", "setting", "samples", "geometric_gpps", "minimum_gpps",
    "target_gpps", "speed_pass_samples", "total_ratio", "ratio_pass_samples",
    "minimum_xpsnr_db", "xpsnr_pass_samples", "exact_samples",
]
tsv_path.parent.mkdir(parents=True, exist_ok=True)
with tsv_path.open("w", newline="") as stream:
    writer = csv.DictWriter(stream, fields, delimiter="\t", extrasaction="ignore")
    writer.writeheader()
    for row in summary + resolution_summary:
        writer.writerow({key: f"{value:.9f}" if isinstance(value, float) else value for key, value in row.items()})

speed_rows = [row for row in summary if row["axis"] != "quality_rate"]
rate_rows = [row for row in summary if row["axis"] == "quality_rate"]
lines = [
    "# CUDA feedback summary",
    "",
    "GP/s counts full-resolution luma pixels. Encode numbers are medians per sample, then a geometric mean across samples. CUDA decode is complete-call timing. Minimums and pass counts expose scaling failures hidden by aggregates.",
    "",
    "## Speed",
    "",
    "| Axis | Q | Setting | Samples | Geo. GP/s | Min GP/s | Target | Passing |",
    "|---|---:|---|---:|---:|---:|---:|---:|",
]
for row in speed_rows:
    lines.append(
        f"| {row['axis']} | {row['quality']} | {row['setting']} | {row['samples']} | "
        f"{row['geometric_gpps']:.6f} | {row['minimum_gpps']:.6f} | "
        f">{row['target_gpps']:.0f} | {row['speed_pass_samples']}/{row['samples']} |"
    )
lines += [
    "",
    "## Rate and quality",
    "",
    "| Q | Samples | Total ratio | >15x | Min Y XPSNR | >50 dB | Exact |",
    "|---:|---:|---:|---:|---:|---:|---:|",
]
for row in rate_rows:
    xpsnr = "inf" if math.isinf(row["minimum_xpsnr_db"]) else f"{row['minimum_xpsnr_db']:.4f} dB"
    lines.append(
        f"| {row['quality']} | {row['samples']} | {row['total_ratio']:.6f}x | "
        f"{row['ratio_pass_samples']}/{row['samples']} | {xpsnr} | "
        f"{row['xpsnr_pass_samples']}/{row['samples']} | {row['exact_samples']}/{row['samples']} |"
    )
hd_speed = [row for row in resolution_summary if "quality_rate" not in row["axis"]]
hd_rate = next(row for row in resolution_summary if "quality_rate" in row["axis"])
lines += [
    "",
    "## 1080p q90 slice",
    "",
    "Fifteen 1920x1080 samples are reported separately because fixed launch and orchestration costs make the 4K-only result non-representative.",
    "",
    "| Axis | Setting | Geo. GP/s | Min GP/s | Passing |",
    "|---|---|---:|---:|---:|",
]
for row in hd_speed:
    lines.append(
        f"| {row['axis']} | {row['setting']} | {row['geometric_gpps']:.6f} | "
        f"{row['minimum_gpps']:.6f} | {row['speed_pass_samples']}/{row['samples']} |"
    )
lines += [
    "",
    f"Q90 totals {hd_rate['total_ratio']:.6f}x compression; {hd_rate['ratio_pass_samples']}/{hd_rate['samples']} samples exceed 15x. Minimum luma XPSNR is {hd_rate['minimum_xpsnr_db']:.4f} dB and {hd_rate['xpsnr_pass_samples']}/{hd_rate['samples']} exceed 50 dB.",
]
lines += [
    "",
    "The four INSTRUCTIONS targets are conjunctive. An aggregate pass does not override a failing sample, and the Rust encode rows are a correctness/reference baseline—not a claim about the not-yet-implemented CUDA encoder.",
    "",
]
markdown_path.parent.mkdir(parents=True, exist_ok=True)
markdown_path.write_text("\n".join(lines))
