#!/usr/bin/env python3
"""Static CUDA-style wavefront occupancy model for Fastvid predictor tiles."""

from __future__ import annotations

import argparse
import csv
import json
import math
from pathlib import Path


def tile_shapes(width: int, height: int) -> list[tuple[int, int]]:
    shapes = []
    for plane in range(3):
        plane_width = width if plane == 0 else (width + 1) // 2
        nominal_width = 256 if plane == 0 else 128
        for y in range(0, height, 128):
            tile_height = min(128, height - y)
            for x in range(0, plane_width, nominal_width):
                shapes.append((min(nominal_width, plane_width - x), tile_height))
    return shapes


def diagonal_lengths(width: int, height: int) -> list[int]:
    return [
        min(diagonal + 1, width, height, width + height - 1 - diagonal)
        for diagonal in range(width + height - 1)
    ]


def model(sample: dict[str, object], bit_depth: int, band_height: int) -> dict[str, object]:
    shapes = tile_shapes(int(sample["width"]), int(sample["height"]))
    units = []
    for width, height in shapes:
        for y in range(0, height, band_height):
            current_height = min(band_height, height - y)
            diagonals = diagonal_lengths(width, current_height)
            warp_slots = sum(math.ceil(length / 32) * 32 for length in diagonals)
            units.append(
                (
                    width * current_height,
                    len(diagonals),
                    max(diagonals),
                    warp_slots,
                )
            )
    total_samples = sum(unit[0] for unit in units)
    total_slots = sum(unit[3] for unit in units)
    storage_bytes = 1 if bit_depth == 8 else 2
    return {
        "sample": sample["id"],
        "width": sample["width"],
        "height": sample["height"],
        "bit_depth": bit_depth,
        "band_height": band_height,
        "work_units": len(units),
        "max_unit_samples": max(unit[0] for unit in units),
        "max_wavefront_rounds": max(unit[1] for unit in units),
        "max_active_lanes": max(unit[2] for unit in units),
        "warp_slot_utilization": total_samples / total_slots,
        "max_shared_source_bytes": max(unit[0] for unit in units) * storage_bytes,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--core", type=Path, default=Path("corpus/manifest.json"))
    parser.add_argument(
        "--high-bit", type=Path, default=Path("corpus/high-bit-manifest.json")
    )
    args = parser.parse_args()

    rows = []
    core = json.loads(args.core.read_text())
    for sample in core["samples"]:
        if sample.get("track") == "codec":
            for band_height in (128, 64):
                rows.append(model(sample, 8, band_height))
    high_bit = json.loads(args.high_bit.read_text())
    for sample in high_bit["samples"]:
        depth = int(
            str(sample["pixel_format"]).removeprefix("yuv422p").removesuffix("le")
        )
        for band_height in (128, 64):
            rows.append(model(sample, depth, band_height))

    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", newline="") as output:
        writer = csv.DictWriter(output, fieldnames=list(rows[0]), delimiter="\t")
        writer.writeheader()
        writer.writerows(rows)


if __name__ == "__main__":
    main()
