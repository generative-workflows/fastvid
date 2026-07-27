#!/usr/bin/env python3
"""Validate and summarize the CPU baseline used to constrain CUDA work."""

from __future__ import annotations

import csv
import math
import statistics
import sys
from collections import defaultdict
from pathlib import Path


def read_tsv(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as source:
        return list(csv.DictReader(source, delimiter="\t"))


def geometric(values: list[float]) -> float:
    if not values or any(value <= 0 for value in values):
        raise ValueError("geometric mean requires positive values")
    return math.exp(sum(math.log(value) for value in values) / len(values))


def mean_metric(values: list[float]) -> float:
    return math.inf if values and all(math.isinf(value) for value in values) else statistics.fmean(values)


def fmt(value: float, digits: int = 6) -> str:
    return "inf" if math.isinf(value) else f"{value:.{digits}f}"


if len(sys.argv) != 6:
    raise SystemExit(
        "usage: summarize-cpu-gpu-baseline.py QUALITY SPEED XPSNR OUTPUT_PREFIX RAW_PREFIX"
    )

quality_path = Path(sys.argv[1])
speed_path = Path(sys.argv[2])
xpsnr_path = Path(sys.argv[3])
output_prefix = Path(sys.argv[4])
raw_prefix = sys.argv[5]
quality_rows = read_tsv(quality_path)
speed_rows = read_tsv(speed_path)
xpsnr_rows = read_tsv(xpsnr_path)

xpsnr = {
    (row["sample"], int(row["quality"])): row
    for row in xpsnr_rows
}
quality = {
    (row["sample"], int(row["quality"])): row
    for row in quality_rows
}
if len(xpsnr) != len(quality):
    raise SystemExit("XPSNR and codec quality matrices have different sizes")

per_quality_path = output_prefix.with_name(output_prefix.name + "-quality.tsv")
aggregate_quality_path = output_prefix.with_name(output_prefix.name + "-quality-summary.tsv")
per_speed_path = output_prefix.with_name(output_prefix.name + "-speed.tsv")
aggregate_speed_path = output_prefix.with_name(output_prefix.name + "-speed-summary.tsv")
markdown_path = output_prefix.with_suffix(".md")
for path in [
    per_quality_path,
    aggregate_quality_path,
    per_speed_path,
    aggregate_speed_path,
    markdown_path,
]:
    path.parent.mkdir(parents=True, exist_ok=True)

quality_fields = [
    "sample",
    "bit_depth",
    "quality",
    "frames",
    "raw_bytes",
    "encoded_bytes",
    "ratio",
    "bits_per_luma_pixel",
    "encoded_stream_mb_s",
    "encoded_stream_mbps",
    "y_psnr",
    "cb_psnr",
    "cr_psnr",
    "y_block_ssim",
    "max_error",
    "xpsnr_y_db",
    "xpsnr_u_db",
    "xpsnr_v_db",
    "xpsnr_min_db",
]
normalized_quality: list[dict[str, str]] = []
for key in sorted(quality, key=lambda item: (item[1], item[0])):
    row = quality[key]
    perceptual = xpsnr[key]
    width_height_frames = int(row["raw_bytes"]) // 4
    bits_per_luma_pixel = int(row["encoded_bytes"]) * 8 / width_height_frames
    normalized = {field: row.get(field, "") for field in quality_fields}
    normalized["bits_per_luma_pixel"] = f"{bits_per_luma_pixel:.9f}"
    for field in ["xpsnr_y_db", "xpsnr_u_db", "xpsnr_v_db", "xpsnr_min_db"]:
        normalized[field] = perceptual[field]
    normalized_quality.append(normalized)

with per_quality_path.open("w", newline="", encoding="utf-8") as destination:
    writer = csv.DictWriter(
        destination, fieldnames=quality_fields, delimiter="\t", lineterminator="\n"
    )
    writer.writeheader()
    writer.writerows(normalized_quality)

quality_summary_fields = [
    "quality",
    "samples",
    "total_ratio",
    "geometric_ratio",
    "mean_bits_per_luma_pixel",
    "mean_encoded_stream_mbps",
    "mean_y_psnr",
    "mean_y_block_ssim",
    "worst_max_error",
    "mean_xpsnr_y_db",
    "mean_xpsnr_min_db",
    "all_exact",
]
quality_summaries: list[dict[str, str]] = []
for quality_value in [60, 75, 90, 95, 100]:
    rows = [row for row in normalized_quality if int(row["quality"]) == quality_value]
    exact = all(int(row["max_error"]) == 0 for row in rows)
    summary = {
        "quality": str(quality_value),
        "samples": str(len(rows)),
        "total_ratio": f"{sum(int(row['raw_bytes']) for row in rows) / sum(int(row['encoded_bytes']) for row in rows):.6f}",
        "geometric_ratio": f"{geometric([float(row['ratio']) for row in rows]):.6f}",
        "mean_bits_per_luma_pixel": f"{statistics.fmean(float(row['bits_per_luma_pixel']) for row in rows):.6f}",
        "mean_encoded_stream_mbps": f"{statistics.fmean(float(row['encoded_stream_mbps']) for row in rows):.6f}",
        "mean_y_psnr": fmt(mean_metric([float(row["y_psnr"]) for row in rows])),
        "mean_y_block_ssim": f"{statistics.fmean(float(row['y_block_ssim']) for row in rows):.9f}",
        "worst_max_error": str(max(int(row["max_error"]) for row in rows)),
        "mean_xpsnr_y_db": fmt(mean_metric([float(row["xpsnr_y_db"]) for row in rows])),
        "mean_xpsnr_min_db": fmt(mean_metric([float(row["xpsnr_min_db"]) for row in rows])),
        "all_exact": str(exact).lower(),
    }
    if quality_value == 100 and not exact:
        raise SystemExit("quality 100 is not exact")
    quality_summaries.append(summary)

with aggregate_quality_path.open("w", newline="", encoding="utf-8") as destination:
    writer = csv.DictWriter(
        destination,
        fieldnames=quality_summary_fields,
        delimiter="\t",
        lineterminator="\n",
    )
    writer.writeheader()
    writer.writerows(quality_summaries)

speed_groups: dict[tuple[str, int, int], list[dict[str, str]]] = defaultdict(list)
for row in speed_rows:
    speed_groups[(row["sample"], int(row["quality"]), int(row["threads"]))].append(row)

speed_fields = [
    "sample",
    "bit_depth",
    "quality",
    "threads",
    "trials",
    "encoded_bytes",
    "ratio",
    "median_encode_mpps",
    "median_decode_mpps",
    "median_encode_gpps",
    "median_decode_gpps",
    "encode_raw_gb_s",
    "decode_raw_gb_s",
    "encode_speedup",
    "decode_speedup",
    "encode_parallel_efficiency",
    "decode_parallel_efficiency",
]
speed_medians: dict[tuple[str, int, int], dict[str, str]] = {}
for key, rows in speed_groups.items():
    sample, quality_value, threads = key
    deterministic_fields = [
        "encoded_bytes",
        "ratio",
        "y_psnr",
        "y_block_ssim",
        "max_error",
    ]
    for field in deterministic_fields:
        if len({row[field] for row in rows}) != 1:
            raise SystemExit(f"non-deterministic {field} for {key}")
    quality_reference = quality[(sample, quality_value)]
    for field in deterministic_fields:
        if rows[0][field] != quality_reference[field]:
            raise SystemExit(f"quality/speed {field} mismatch for {key}")
    encode = statistics.median(float(row["encode_mpps"]) for row in rows)
    decode = statistics.median(float(row["decode_mpps"]) for row in rows)
    speed_medians[key] = {
        "sample": sample,
        "bit_depth": rows[0]["bit_depth"],
        "quality": str(quality_value),
        "threads": str(threads),
        "trials": str(len(rows)),
        "encoded_bytes": rows[0]["encoded_bytes"],
        "ratio": rows[0]["ratio"],
        "median_encode_mpps": f"{encode:.6f}",
        "median_decode_mpps": f"{decode:.6f}",
        "median_encode_gpps": f"{encode / 1000:.9f}",
        "median_decode_gpps": f"{decode / 1000:.9f}",
        "encode_raw_gb_s": f"{encode * 0.004:.6f}",
        "decode_raw_gb_s": f"{decode * 0.004:.6f}",
    }

for key, row in speed_medians.items():
    sample, quality_value, threads = key
    baseline = speed_medians[(sample, quality_value, 1)]
    encode_speedup = float(row["median_encode_mpps"]) / float(baseline["median_encode_mpps"])
    decode_speedup = float(row["median_decode_mpps"]) / float(baseline["median_decode_mpps"])
    row["encode_speedup"] = f"{encode_speedup:.6f}"
    row["decode_speedup"] = f"{decode_speedup:.6f}"
    row["encode_parallel_efficiency"] = f"{encode_speedup / threads:.6f}"
    row["decode_parallel_efficiency"] = f"{decode_speedup / threads:.6f}"

with per_speed_path.open("w", newline="", encoding="utf-8") as destination:
    writer = csv.DictWriter(
        destination, fieldnames=speed_fields, delimiter="\t", lineterminator="\n"
    )
    writer.writeheader()
    for key in sorted(speed_medians, key=lambda item: (item[1], item[2], item[0])):
        writer.writerow(speed_medians[key])

speed_summary_fields = [
    "quality",
    "threads",
    "samples",
    "geometric_encode_mpps",
    "geometric_decode_mpps",
    "geometric_encode_gpps",
    "geometric_decode_gpps",
    "encode_raw_gb_s",
    "decode_raw_gb_s",
    "encode_speedup",
    "decode_speedup",
    "encode_parallel_efficiency",
    "decode_parallel_efficiency",
]
speed_summaries: list[dict[str, str]] = []
aggregate_lookup: dict[tuple[int, int], tuple[float, float]] = {}
for quality_value in [90, 100]:
    for threads in [1, 2, 4]:
        rows = [
            row
            for (sample, quality_key, thread_key), row in speed_medians.items()
            if quality_key == quality_value and thread_key == threads
        ]
        encode = geometric([float(row["median_encode_mpps"]) for row in rows])
        decode = geometric([float(row["median_decode_mpps"]) for row in rows])
        aggregate_lookup[(quality_value, threads)] = (encode, decode)
        baseline_encode, baseline_decode = aggregate_lookup[(quality_value, 1)]
        encode_speedup = encode / baseline_encode
        decode_speedup = decode / baseline_decode
        speed_summaries.append(
            {
                "quality": str(quality_value),
                "threads": str(threads),
                "samples": str(len(rows)),
                "geometric_encode_mpps": f"{encode:.6f}",
                "geometric_decode_mpps": f"{decode:.6f}",
                "geometric_encode_gpps": f"{encode / 1000:.9f}",
                "geometric_decode_gpps": f"{decode / 1000:.9f}",
                "encode_raw_gb_s": f"{encode * 0.004:.6f}",
                "decode_raw_gb_s": f"{decode * 0.004:.6f}",
                "encode_speedup": f"{encode_speedup:.6f}",
                "decode_speedup": f"{decode_speedup:.6f}",
                "encode_parallel_efficiency": f"{encode_speedup / threads:.6f}",
                "decode_parallel_efficiency": f"{decode_speedup / threads:.6f}",
            }
        )

with aggregate_speed_path.open("w", newline="", encoding="utf-8") as destination:
    writer = csv.DictWriter(
        destination,
        fieldnames=speed_summary_fields,
        delimiter="\t",
        lineterminator="\n",
    )
    writer.writeheader()
    writer.writerows(speed_summaries)

with markdown_path.open("w", encoding="utf-8") as destination:
    destination.write("# Version-5 CPU baseline for CUDA\n\n")
    destination.write(
        "All rows are all-intra on the checksummed native high-bit corpus. "
        "Rate and quality are deterministic one-thread rows; timing is the "
        "per-sample median of five post-warm-up trials and then a geometric "
        "mean across samples. GP/s counts full-resolution luma pixels.\n\n"
    )
    destination.write("## Rate and quality\n\n")
    destination.write(
        "| Q | Total ratio | Geo. ratio | Bits/luma px | Mean bitrate | "
        "Mean Y PSNR | Mean block SSIM | Mean Y XPSNR | Worst error | Exact |\n"
    )
    destination.write("|---:|---:|---:|---:|---:|---:|---:|---:|---:|:---:|\n")
    for row in quality_summaries:
        destination.write(
            f"| {row['quality']} | {row['total_ratio']}x | "
            f"{row['geometric_ratio']}x | {row['mean_bits_per_luma_pixel']} | "
            f"{row['mean_encoded_stream_mbps']} Mb/s | {row['mean_y_psnr']} dB | "
            f"{row['mean_y_block_ssim']} | {row['mean_xpsnr_y_db']} dB | "
            f"{row['worst_max_error']} | {row['all_exact']} |\n"
        )
    destination.write("\n## Speed and thread scaling\n\n")
    destination.write(
        "| Q | Threads | Encode GP/s | Decode GP/s | Encode raw GB/s | "
        "Decode raw GB/s | Encode scaling | Decode scaling | Enc. efficiency |\n"
    )
    destination.write("|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n")
    for row in speed_summaries:
        destination.write(
            f"| {row['quality']} | {row['threads']} | "
            f"{row['geometric_encode_gpps']} | {row['geometric_decode_gpps']} | "
            f"{row['encode_raw_gb_s']} | {row['decode_raw_gb_s']} | "
            f"{row['encode_speedup']}x | {row['decode_speedup']}x | "
            f"{float(row['encode_parallel_efficiency']) * 100:.1f}% |\n"
        )
    destination.write("\n## Provenance\n\n")
    destination.write(
        f"- Raw quality: `{raw_prefix}-quality.tsv`\n"
        f"- Raw speed: `{raw_prefix}-speed.tsv`\n"
        f"- Raw XPSNR: `{raw_prefix}-xpsnr.tsv`\n"
        f"- Environment: `{raw_prefix}-environment.txt`\n"
        f"- Per-sample normalized tables: `{per_quality_path}` and `{per_speed_path}`\n"
    )
    destination.write(
        "\nXPSNR was the deepest reproducible metric available on this host. "
        "This FFmpeg build lacks libvmaf, and the environment lacks pinned "
        "DISTS/ColorVideoVDP dependencies. The four native inputs are "
        "procedural, so this is a regression and GPU-handoff baseline rather "
        "than a natural-content subjective-quality claim.\n\n"
        "OpenAPV remains in the separate matched native-10-bit external panel "
        "at `benchmarks/openapv-frontier-summary.tsv`; its preserved rows are "
        "not pooled into this four-sample cross-depth aggregate.\n"
    )

print(f"quality summary: {aggregate_quality_path}")
print(f"speed summary: {aggregate_speed_path}")
print(f"report: {markdown_path}")
