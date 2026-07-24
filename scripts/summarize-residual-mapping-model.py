#!/usr/bin/env python3
"""Summarize EXP-0046 with complete-stream and category accounting."""

import csv
import statistics
import sys
from collections import defaultdict


HEADER_BYTES = 32
DIRECTORY_ENTRY_BYTES = 32


def category(sample: str) -> str:
    if sample.startswith(("bbb-", "ed-")):
        return "natural-cinema"
    if sample.startswith(("camera-", "noisy-camera-")):
        return "camera"
    if sample.startswith("ai-"):
        return "ai-generated"
    if sample.startswith("hdr-gradient-"):
        return "hdr-gradient"
    if sample.startswith("high-precision-motion-"):
        return "high-precision-motion"
    if sample.startswith(
        ("ui-", "procedural-", "resolution-", "high-precision-ui-")
    ):
        return "synthetic-ui"
    raise ValueError(f"unclassified sample: {sample}")


def payloads(rows: list[dict[str, str]]) -> tuple[int, int, int]:
    return (
        sum(int(row["actual_bytes"]) for row in rows),
        sum(int(row["bounded_bytes"]) for row in rows),
        sum(int(row["oracle_bytes"]) for row in rows),
    )


def delta(candidate: int, baseline: int) -> float:
    return candidate / baseline - 1


def main() -> None:
    with open(sys.argv[1], encoding="utf-8", newline="") as source:
        rows = list(csv.DictReader(source, delimiter="\t"))
    if not rows:
        raise SystemExit("no model rows")

    actual, bounded, oracle = payloads(rows)
    frames = {
        (row["sample"], row["frame"], row["bit_depth"], row["quality"], row["gop"])
        for row in rows
    }
    overhead = len(frames) * HEADER_BYTES + len(rows) * DIRECTORY_ENTRY_BYTES
    complete_actual = actual + overhead
    complete_oracle = oracle + overhead
    wins = sum(
        int(row["bounded_bytes"]) < int(row["actual_bytes"]) for row in rows
    )
    five_percent_wins = sum(
        int(row["bounded_bytes"]) * 100 <= int(row["actual_bytes"]) * 95
        for row in rows
    )
    tile_deltas = [
        delta(int(row["bounded_bytes"]), int(row["actual_bytes"]))
        for row in rows
        if int(row["actual_bytes"]) != 0
    ]

    print(f"tiles={len(rows)} frames={len(frames)} overhead={overhead}")
    print(
        f"payload actual={actual} bounded={bounded} oracle={oracle} "
        f"bounded_delta={delta(bounded, actual):+.2%} "
        f"oracle_delta={delta(oracle, actual):+.2%}"
    )
    print(
        f"complete_stream actual={complete_actual} oracle={complete_oracle} "
        f"oracle_delta={delta(complete_oracle, complete_actual):+.2%}"
    )
    print(
        f"winning_tiles={wins}/{len(rows)} ({wins / len(rows):.2%}) "
        f"5%-winning_tiles={five_percent_wins}/{len(rows)} "
        f"({five_percent_wins / len(rows):.2%}) "
        f"median_tile_delta={statistics.median(tile_deltas):+.2%}"
    )

    grouped: dict[tuple[str, str], list[dict[str, str]]] = defaultdict(list)
    for row in rows:
        grouped[("category", category(row["sample"]))].append(row)
        grouped[("bit-depth", row["bit_depth"])].append(row)
        grouped[("quality", row["quality"])].append(row)
        grouped[("prediction", row["prediction"])].append(row)
        grouped[("source-entropy", row["source_entropy"])].append(row)
        grouped[("sample", row["sample"])].append(row)

    results = []
    for group, group_rows in grouped.items():
        group_actual, group_bounded, group_oracle = payloads(group_rows)
        results.append(
            (
                group,
                len(group_rows),
                delta(group_bounded, group_actual),
                delta(group_oracle, group_actual),
            )
        )

    for group_type in [
        "category",
        "bit-depth",
        "quality",
        "prediction",
        "source-entropy",
    ]:
        print(f"{group_type}:")
        for group, count, bounded_delta, oracle_delta in sorted(
            result for result in results if result[0][0] == group_type
        ):
            print(
                f"  {group[1]} tiles={count} bounded={bounded_delta:+.2%} "
                f"oracle={oracle_delta:+.2%}"
            )

    category_results = [
        result for result in results if result[0][0] == "category"
    ]
    gate = (
        delta(complete_oracle, complete_actual) <= -0.02
        and sum(result[3] <= -0.01 for result in category_results) >= 2
    )
    print(f"prototype_gate={'PASS' if gate else 'FAIL'}")


if __name__ == "__main__":
    main()
