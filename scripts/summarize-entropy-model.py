#!/usr/bin/env python3
"""Summarize EXP-0038 with fully charged payload sizes."""

import csv
import statistics
import sys
from collections import defaultdict


VARIANTS = {
    "stream-vbyte": "stream_vbyte_bytes",
    "stream-vbyte-0124": "stream_vbyte_0124_bytes",
}


def summarize(rows: list[dict[str, str]], key: str) -> tuple[int, int, int, int]:
    actual = sum(int(row["actual_bytes"]) for row in rows)
    modeled = sum(int(row[key]) for row in rows)
    wins = sum(int(row[key]) * 100 <= int(row["actual_bytes"]) * 95 for row in rows)
    oracle = sum(min(int(row["actual_bytes"]), int(row[key])) for row in rows)
    return actual, modeled, wins, oracle


def delta(modeled: int, actual: int) -> float:
    return modeled / actual - 1


def main() -> None:
    with open(sys.argv[1], encoding="utf-8", newline="") as source:
        rows = list(csv.DictReader(source, delimiter="\t"))
    print(f"tiles: {len(rows)}")

    for name, key in VARIANTS.items():
        actual, modeled, wins, oracle = summarize(rows, key)
        tile_deltas = [
            delta(int(row[key]), int(row["actual_bytes"])) for row in rows
        ]
        gate_a = delta(modeled, actual) <= 0.02 and wins / len(rows) >= 0.20
        print(
            f"{name}: actual={actual} modeled={modeled} "
            f"delta={delta(modeled, actual):+.2%} "
            f"5%-wins={wins}/{len(rows)} ({wins / len(rows):.2%}) "
            f"oracle_delta={delta(oracle, actual):+.2%} "
            f"median_tile_delta={statistics.median(tile_deltas):+.2%} "
            f"gate_A={'PASS' if gate_a else 'FAIL'}"
        )

        grouped: dict[tuple[str, str], list[dict[str, str]]] = defaultdict(list)
        for row in rows:
            grouped[("bit-depth", row["bit_depth"])].append(row)
            grouped[("quality", row["quality"])].append(row)
            grouped[("prediction", row["prediction"])].append(row)
            grouped[("source-entropy", row["source_entropy"])].append(row)
            grouped[("sample", row["sample"])].append(row)

        group_results = []
        for group, group_rows in grouped.items():
            group_actual, group_modeled, group_wins, group_oracle = summarize(
                group_rows, key
            )
            group_results.append(
                (
                    delta(group_modeled, group_actual),
                    group,
                    len(group_rows),
                    group_wins,
                    delta(group_oracle, group_actual),
                )
            )
        print("  best aggregate groups:")
        for group_delta, group, count, group_wins, oracle_delta in sorted(
            group_results
        )[:8]:
            print(
                f"    {group[0]}={group[1]} delta={group_delta:+.2%} "
                f"tiles={count} 5%-wins={group_wins} "
                f"oracle_delta={oracle_delta:+.2%}"
            )

        bit_depth_groups = [
            result for result in group_results if result[1][0] == "bit-depth"
        ]
        gate_b = (
            any(result[0] <= -0.05 for result in bit_depth_groups)
            and all(result[0] <= 0.05 for result in bit_depth_groups)
        )
        print(f"  gate_B_bit_depth={'PASS' if gate_b else 'FAIL'}")


if __name__ == "__main__":
    main()
