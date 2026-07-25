#!/usr/bin/env python3
"""Validate and summarize the charged reversible-squeeze model."""

from __future__ import annotations

import csv
import sys
from collections import Counter, defaultdict
from pathlib import Path


def summarize(rows: list[dict[str, str]], keys: tuple[str, ...]) -> None:
    groups: dict[tuple[str, ...], list[dict[str, str]]] = defaultdict(list)
    for row in rows:
        groups[tuple(row[key] for key in keys)].append(row)
    for group_key in sorted(groups):
        group = groups[group_key]
        current = sum(int(row["current_bytes"]) for row in group)
        best = sum(int(row["best_bytes"]) for row in group)
        winners = Counter(row["winner"] for row in group)
        selected = len(group) - winners["current"]
        label = "/".join(group_key)
        print(
            f"{label}\t{len(group)}\t{current}\t{best}\t"
            f"{100 * (best / current - 1):+.3f}\t{selected}\t"
            f"{winners['horizontal']}\t{winners['vertical']}\t"
            f"{winners['two-dimensional']}"
        )


def main() -> int:
    if len(sys.argv) != 2:
        raise SystemExit(f"usage: {Path(sys.argv[0]).name} RESULTS.tsv")
    with Path(sys.argv[1]).open(newline="") as source:
        rows = list(csv.DictReader(source, delimiter="\t"))
    if not rows:
        raise SystemExit("no model rows")

    identity = Counter(
        (row["sample"], row["frame"], row["plane"], row["tile"]) for row in rows
    )
    duplicates = [key for key, count in identity.items() if count != 1]
    if duplicates:
        raise SystemExit(f"duplicate model rows: {duplicates[:3]}")
    for row in rows:
        values = [
            int(row["current_bytes"]),
            int(row["horizontal_bytes"]),
            int(row["vertical_bytes"]),
            int(row["two_dimensional_bytes"]),
        ]
        expected = min(values)
        if int(row["best_bytes"]) != expected:
            raise SystemExit(f"incorrect best-byte fallback in row {row}")
        winner_index = values.index(expected)
        expected_winner = (
            "current",
            "horizontal",
            "vertical",
            "two-dimensional",
        )[winner_index]
        if row["winner"] != expected_winner:
            raise SystemExit(f"incorrect winner in row {row}")

    print(
        "group\ttiles\tcurrent_bytes\tbest_bytes\tdelta_pct\tselected_tiles\t"
        "horizontal_wins\tvertical_wins\ttwo_dimensional_wins"
    )
    summarize(rows, ("sample",))
    summarize(rows, ("bit_depth",))
    summarize(rows, ("plane",))
    summarize(rows, ())
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
