#!/usr/bin/env python3
"""Summarize EXP-0106 raster-versus-diagonal entropy-order sizes."""

import csv
import sys
from collections import defaultdict


def percent_delta(candidate: int, baseline: int) -> float:
    return candidate / baseline - 1 if baseline else 0.0


def totals(rows: list[dict[str, str]]) -> tuple[int, int]:
    return (
        sum(int(row["raster_best_bytes"]) for row in rows),
        sum(int(row["diagonal_best_bytes"]) for row in rows),
    )


def main() -> None:
    with open(sys.argv[1], encoding="utf-8", newline="") as source:
        rows = [
            row
            for row in csv.DictReader(source, delimiter="\t")
            if row["diagonal_order_supported"] == "true"
        ]

    grouped: dict[str, list[dict[str, str]]] = defaultdict(list)
    for row in rows:
        grouped[row["sample"]].append(row)

    rice_mismatches = sum(
        int(row["diagonal_rice_bytes"]) != int(row["raster_rice_bytes"])
        for row in rows
    )
    raster, diagonal = totals(rows)
    sample_results = []
    for sample, sample_rows in grouped.items():
        sample_raster, sample_diagonal = totals(sample_rows)
        sample_results.append(
            (percent_delta(sample_diagonal, sample_raster), sample,
             sample_raster, sample_diagonal)
        )

    aggregate_delta = percent_delta(diagonal, raster)
    worst_delta = max(delta for delta, *_ in sample_results)
    best_delta = min(delta for delta, *_ in sample_results)
    passed = (
        rice_mismatches == 0
        and aggregate_delta <= 0.005
        and worst_delta <= 0.02
        and best_delta <= -0.01
    )

    print(f"spatial tiles: {len(rows)}")
    print(
        f"aggregate: raster={raster} diagonal={diagonal} "
        f"delta={aggregate_delta:+.4%}"
    )
    print(
        f"rice order mismatches={rice_mismatches}"
    )
    for delta, sample, sample_raster, sample_diagonal in sorted(sample_results):
        print(
            f"{sample}: raster={sample_raster} diagonal={sample_diagonal} "
            f"delta={delta:+.4%}"
        )
    print(f"gate={'PASS' if passed else 'FAIL'}")


if __name__ == "__main__":
    main()
