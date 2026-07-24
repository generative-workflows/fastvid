#!/usr/bin/env python3
"""Aggregate rectangular-tile screens relative to the 256x128 default."""

from __future__ import annotations

import argparse
import csv
import math
import statistics
from collections import defaultdict
from pathlib import Path


def geometric_mean(values: list[float]) -> float:
    if not values or any(value <= 0 for value in values):
        raise ValueError("geometric mean needs positive values")
    return math.exp(sum(math.log(value) for value in values) / len(values))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("results")
    parser.add_argument("summary")
    args = parser.parse_args()

    with Path(args.results).open(newline="", encoding="utf-8") as source:
        rows = list(csv.DictReader(source, delimiter="\t"))
    if not rows:
        raise ValueError("tile-geometry results are empty")

    grouped: dict[tuple[int, int, str], list[dict[str, str]]] = defaultdict(list)
    cases = sorted({row["case"] for row in rows})
    for row in rows:
        geometry = (int(row["tile_width"]), int(row["tile_height"]))
        grouped[(geometry[0], geometry[1], row["case"])].append(row)

    geometries = sorted({(key[0], key[1]) for key in grouped})
    points: dict[tuple[int, int], dict[str, float]] = {}
    quality: dict[tuple[int, int], dict[str, dict[str, float]]] = {}
    for width, height in geometries:
        medians: list[dict[str, float]] = []
        quality[(width, height)] = {}
        total_bytes = 0
        total_tiles = 0
        for case in cases:
            group = grouped.get((width, height, case), [])
            if not group:
                raise ValueError(f"missing {width}x{height}/{case}")
            encoded = {int(row["encoded_bytes"]) for row in group}
            if len(encoded) != 1:
                raise ValueError(f"encoded size changed for {width}x{height}/{case}")
            total_bytes += encoded.pop()
            total_tiles += round(
                statistics.median(
                    int(row["zero_run_tiles"]) + int(row["rice_tiles"]) for row in group
                )
            )
            quality[(width, height)][case] = {
                "y_psnr": statistics.median(float(row["y_psnr"]) for row in group),
                "y_ssim": statistics.median(
                    float(row["y_block_ssim"]) for row in group
                ),
                "max_error": statistics.median(
                    float(row["max_error"]) for row in group
                ),
            }
            medians.append(
                {
                    "ratio": statistics.median(float(row["ratio"]) for row in group),
                    "encode": statistics.median(
                        float(row["encode_mpps"]) for row in group
                    ),
                    "decode": statistics.median(
                        float(row["decode_mpps"]) for row in group
                    ),
                    "encode_mb_s": statistics.median(
                        float(row["encode_raw_mb_s"]) for row in group
                    ),
                    "decode_mb_s": statistics.median(
                        float(row["decode_raw_mb_s"]) for row in group
                    ),
                    "bitrate": statistics.median(
                        float(row["encoded_stream_mbps"]) for row in group
                    ),
                }
            )
        points[(width, height)] = {
            "bytes": float(total_bytes),
            "ratio": geometric_mean([row["ratio"] for row in medians]),
            "encode": geometric_mean([row["encode"] for row in medians]),
            "decode": geometric_mean([row["decode"] for row in medians]),
            "encode_mb_s": geometric_mean([row["encode_mb_s"] for row in medians]),
            "decode_mb_s": geometric_mean([row["decode_mb_s"] for row in medians]),
            "bitrate": geometric_mean([row["bitrate"] for row in medians]),
            "tiles": float(total_tiles),
        }

    baseline = points.get((256, 128))
    if baseline is None:
        raise ValueError("256x128 baseline is missing")
    fields = [
        "tile_width",
        "tile_height",
        "area_vs_default",
        "encoded_bytes",
        "bytes_delta_pct",
        "ratio",
        "encode_mpps",
        "encode_delta_pct",
        "decode_mpps",
        "decode_delta_pct",
        "encode_raw_mb_s",
        "decode_raw_mb_s",
        "encoded_stream_mbps",
        "total_tiles",
        "worst_y_psnr_delta_db",
        "worst_y_ssim_delta",
        "maximum_error",
    ]
    with Path(args.summary).open("w", newline="", encoding="utf-8") as target:
        writer = csv.DictWriter(
            target, fieldnames=fields, delimiter="\t", lineterminator="\n"
        )
        writer.writeheader()
        for (width, height), point in points.items():
            psnr_deltas = [
                quality[(width, height)][case]["y_psnr"]
                - quality[(256, 128)][case]["y_psnr"]
                for case in cases
                if math.isfinite(quality[(256, 128)][case]["y_psnr"])
            ]
            ssim_deltas = [
                quality[(width, height)][case]["y_ssim"]
                - quality[(256, 128)][case]["y_ssim"]
                for case in cases
            ]
            writer.writerow(
                {
                    "tile_width": width,
                    "tile_height": height,
                    "area_vs_default": f"{width * height / (256 * 128):.3f}",
                    "encoded_bytes": round(point["bytes"]),
                    "bytes_delta_pct": f"{100 * (point['bytes'] / baseline['bytes'] - 1):.3f}",
                    "ratio": f"{point['ratio']:.6f}",
                    "encode_mpps": f"{point['encode']:.6f}",
                    "encode_delta_pct": f"{100 * (point['encode'] / baseline['encode'] - 1):.3f}",
                    "decode_mpps": f"{point['decode']:.6f}",
                    "decode_delta_pct": f"{100 * (point['decode'] / baseline['decode'] - 1):.3f}",
                    "encode_raw_mb_s": f"{point['encode_mb_s']:.6f}",
                    "decode_raw_mb_s": f"{point['decode_mb_s']:.6f}",
                    "encoded_stream_mbps": f"{point['bitrate']:.6f}",
                    "total_tiles": round(point["tiles"]),
                    "worst_y_psnr_delta_db": f"{min(psnr_deltas):.6f}",
                    "worst_y_ssim_delta": f"{min(ssim_deltas):.8f}",
                    "maximum_error": round(
                        max(
                            quality[(width, height)][case]["max_error"]
                            for case in cases
                        )
                    ),
                }
            )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
