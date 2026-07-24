#!/usr/bin/env python3
"""Summarize EXP-0071 charged chroma-from-luma model output."""

from __future__ import annotations

import csv
import sys
from collections import defaultdict
from pathlib import Path


def saved(candidate: int, baseline: int) -> float:
    return (1.0 - candidate / baseline) * 100.0


def summarize(rows: list[dict[str, str]]) -> dict[str, int]:
    current = sum(int(row["current_bytes"]) for row in rows)
    oracle = sum(
        min(int(row["current_bytes"]), int(row["cfl_complete_bytes"]))
        for row in rows
    )
    selected = sum(
        int(row["cfl_complete_bytes"]) < int(row["current_bytes"]) for row in rows
    )
    frames: dict[tuple[str, str, str], int] = {}
    frame_savings: dict[tuple[str, str, str], int] = defaultdict(int)
    for row in rows:
        key = (row["sample"], row["frame"], row["quality"])
        frames[key] = int(row["current_stream_bytes"])
        frame_savings[key] += max(
            0, int(row["current_bytes"]) - int(row["cfl_complete_bytes"])
        )
    stream = sum(frames.values())
    oracle_stream = stream - sum(frame_savings.values())
    return {
        "tiles": len(rows),
        "selected": selected,
        "current": current,
        "oracle": oracle,
        "stream": stream,
        "oracle_stream": oracle_stream,
    }


def print_group(name: str, rows: list[dict[str, str]]) -> None:
    result = summarize(rows)
    print(
        f"{name}\t{result['tiles']}\t{result['selected']}"
        f"\t{result['current']}\t{result['oracle']}"
        f"\t{saved(result['oracle'], result['current']):.3f}"
        f"\t{result['stream']}\t{result['oracle_stream']}"
        f"\t{saved(result['oracle_stream'], result['stream']):.3f}"
    )


def main() -> int:
    if len(sys.argv) != 2:
        raise SystemExit(f"usage: {Path(sys.argv[0]).name} RESULTS.tsv")
    with Path(sys.argv[1]).open(newline="", encoding="utf-8") as source:
        rows = list(csv.DictReader(source, delimiter="\t"))
    if not rows:
        raise SystemExit("no model rows")
    print(
        "group\ttiles\tselected\tcurrent_chroma\toracle_chroma"
        "\tchroma_saved_pct\tcurrent_stream\toracle_stream\tstream_saved_pct"
    )
    dimensions = [
        ("sample", lambda row: row["sample"]),
        ("category", lambda row: row["category"]),
        ("plane", lambda row: row["plane"]),
        ("quality", lambda row: row["quality"]),
    ]
    for label, key_function in dimensions:
        groups: dict[str, list[dict[str, str]]] = defaultdict(list)
        for row in rows:
            groups[key_function(row)].append(row)
        for name, group in sorted(groups.items()):
            print_group(f"{label}:{name}", group)
    print_group("TOTAL", rows)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
