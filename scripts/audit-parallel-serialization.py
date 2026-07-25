#!/usr/bin/env python3
"""Model Fastvid access-tile and hypothetical entropy-shard serialization."""

from __future__ import annotations

import argparse
import csv
import json
import math
from pathlib import Path


DEFAULT_SHARDS = (256, 512, 1024, 2048, 4096)
TILE_WIDTH = 256
TILE_HEIGHT = 128


def tile_sample_counts(width: int, height: int) -> list[int]:
    counts: list[int] = []
    for plane in range(3):
        plane_width = width if plane == 0 else (width + 1) // 2
        nominal_width = TILE_WIDTH if plane == 0 else (TILE_WIDTH + 1) // 2
        for y in range(0, height, TILE_HEIGHT):
            tile_height = min(TILE_HEIGHT, height - y)
            for x in range(0, plane_width, nominal_width):
                tile_width = min(nominal_width, plane_width - x)
                counts.append(tile_width * tile_height)
    return counts


def load_samples(manifest: Path, high_bit: bool) -> list[dict[str, object]]:
    document = json.loads(manifest.read_text())
    samples = []
    for sample in document["samples"]:
        if not high_bit and sample.get("track") != "codec":
            continue
        pixel_format = str(sample.get("pixel_format", "yuv422p8"))
        if not pixel_format.startswith("yuv422p"):
            continue
        samples.append(sample)
    return samples


def rows_for_sample(
    sample: dict[str, object], high_bit: bool, shard_sizes: tuple[int, ...]
) -> list[dict[str, object]]:
    width = int(sample["width"])
    height = int(sample["height"])
    counts = tile_sample_counts(width, height)
    luma_pixels = width * height
    total_samples = sum(counts)
    bit_depth = (
        int(str(sample["pixel_format"]).removeprefix("yuv422p").removesuffix("le"))
        if high_bit
        else 8
    )
    output = []
    for shard_symbols in shard_sizes:
        shards = sum(math.ceil(count / shard_symbols) for count in counts)
        extra_boundaries = shards - len(counts)
        u16_bytes = extra_boundaries * 2
        u32_bytes = extra_boundaries * 4
        worst_padding_bytes = extra_boundaries
        output.append(
            {
                "sample": sample["id"],
                "width": width,
                "height": height,
                "bit_depth": bit_depth,
                "luma_pixels": luma_pixels,
                "total_samples": total_samples,
                "access_tiles": len(counts),
                "access_tiles_per_mp": len(counts) * 1_000_000 / luma_pixels,
                "max_tile_samples": max(counts),
                "current_max_entropy_span": max(counts),
                "shard_symbols": shard_symbols,
                "execution_shards": shards,
                "execution_shards_per_mp": shards * 1_000_000 / luma_pixels,
                "max_shard_symbols": min(shard_symbols, max(counts)),
                "extra_boundaries": extra_boundaries,
                "u16_length_bytes": u16_bytes,
                "u32_length_bytes": u32_bytes,
                "worst_padding_bytes": worst_padding_bytes,
                "u32_plus_padding_bpp": (
                    (u32_bytes + worst_padding_bytes) * 8 / luma_pixels
                ),
            }
        )
    return output


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--core-manifest", type=Path, default=Path("corpus/manifest.json")
    )
    parser.add_argument(
        "--high-bit-manifest",
        type=Path,
        default=Path("corpus/high-bit-manifest.json"),
    )
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument(
        "--shard-symbols",
        type=int,
        nargs="+",
        default=list(DEFAULT_SHARDS),
    )
    args = parser.parse_args()

    shard_sizes = tuple(sorted(set(args.shard_symbols)))
    if not shard_sizes or shard_sizes[0] <= 0:
        raise SystemExit("shard sizes must be positive")

    rows = []
    for sample in load_samples(args.core_manifest, high_bit=False):
        rows.extend(rows_for_sample(sample, False, shard_sizes))
    for sample in load_samples(args.high_bit_manifest, high_bit=True):
        rows.extend(rows_for_sample(sample, True, shard_sizes))

    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", newline="") as output:
        writer = csv.DictWriter(output, fieldnames=list(rows[0]), delimiter="\t")
        writer.writeheader()
        writer.writerows(rows)


if __name__ == "__main__":
    main()
