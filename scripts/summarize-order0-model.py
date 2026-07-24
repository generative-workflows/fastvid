#!/usr/bin/env python3
"""Summarize EXP-0053 complete-byte order-0 model output."""

from __future__ import annotations

import csv
import sys
from collections import Counter, defaultdict
from pathlib import Path

HEADER_BYTES = 32
DIRECTORY_ENTRY_BYTES = 32


def percent(candidate: int, baseline: int) -> float:
    return (candidate / baseline - 1.0) * 100.0


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


def summarize(rows: list[dict[str, str]]) -> dict[str, int]:
    frames = {
        (
            row["sample"],
            row["frame"],
            row["bit_depth"],
            row["quality"],
            row["gop"],
        )
        for row in rows
    }
    actual_payload = sum(int(row["actual_bytes"]) for row in rows)
    ideal_payload = sum(int(row["ideal_order0_bytes"]) for row in rows)
    modeled_payload = 0
    winning_table_bytes = 0
    wins = 0
    unsupported = 0
    for row in rows:
        actual = int(row["actual_bytes"])
        supported = row["order0_supported"] == "true"
        unsupported += not supported
        modeled = int(row["order0_complete_bytes"]) if supported else actual
        if modeled < actual:
            modeled_payload += modeled
            winning_table_bytes += int(row["order0_table_bytes"])
            wins += 1
        else:
            modeled_payload += actual
    overhead = len(frames) * HEADER_BYTES + len(rows) * DIRECTORY_ENTRY_BYTES
    return {
        "tiles": len(rows),
        "frames": len(frames),
        "actual": actual_payload + overhead,
        "ideal": ideal_payload + overhead,
        "modeled": modeled_payload + overhead,
        "wins": wins,
        "unsupported": unsupported,
        "table": winning_table_bytes,
    }


def main() -> int:
    if len(sys.argv) != 2:
        raise SystemExit(f"usage: {Path(sys.argv[0]).name} RESULTS.tsv")
    with Path(sys.argv[1]).open(newline="", encoding="utf-8") as source:
        rows = list(csv.DictReader(source, delimiter="\t"))
    if not rows:
        raise SystemExit("no model rows")

    groups: dict[str, list[dict[str, str]]] = defaultdict(list)
    for row in rows:
        groups[row["sample"]].append(row)

    print(
        "group\ttiles\tframes\tactual_complete\tideal_complete\tideal_delta_pct"
        "\tmodeled_oracle_complete\tmodeled_delta_pct\twinning_tiles"
        "\tunsupported_tiles\twinning_table_bytes"
    )
    for name, group in [*sorted(groups.items()), ("TOTAL", rows)]:
        result = summarize(group)
        print(
            f"{name}\t{result['tiles']}\t{result['frames']}\t{result['actual']}"
            f"\t{result['ideal']}\t{percent(result['ideal'], result['actual']):.3f}"
            f"\t{result['modeled']}\t{percent(result['modeled'], result['actual']):.3f}"
            f"\t{result['wins']}\t{result['unsupported']}\t{result['table']}"
        )

    category_groups: dict[str, list[dict[str, str]]] = defaultdict(list)
    for row in rows:
        category_groups[category(row["sample"])].append(row)
    print("categories:")
    for name, group in sorted(category_groups.items()):
        result = summarize(group)
        print(
            f"  {name}: tiles={result['tiles']} "
            f"ideal={percent(result['ideal'], result['actual']):+.3f}% "
            f"modeled={percent(result['modeled'], result['actual']):+.3f}% "
            f"wins={result['wins']}"
        )

    for field in ["bit_depth", "quality", "prediction", "source_entropy"]:
        field_groups: dict[str, list[dict[str, str]]] = defaultdict(list)
        for row in rows:
            field_groups[row[field]].append(row)
        print(f"{field}:")
        for name, group in sorted(field_groups.items()):
            actual = sum(int(row["actual_bytes"]) for row in group)
            modeled = sum(
                min(
                    int(row["actual_bytes"]),
                    int(row["order0_complete_bytes"])
                    if row["order0_supported"] == "true"
                    else int(row["actual_bytes"]),
                )
                for row in group
            )
            print(
                f"  {name}: tiles={len(group)} "
                f"payload={percent(modeled, actual):+.3f}%"
            )

    distinct = sorted(int(row["distinct_symbols"]) for row in rows)

    def nearest_rank(fraction: float) -> int:
        return distinct[max(0, int(len(distinct) * fraction + 0.999999) - 1)]

    winning_rows = [
        row
        for row in rows
        if row["order0_supported"] == "true"
        and int(row["order0_complete_bytes"]) < int(row["actual_bytes"])
    ]
    table_logs = Counter(row["order0_table_log"] for row in winning_rows)
    print(
        "distinct_symbols: "
        f"p50={nearest_rank(0.50)} p95={nearest_rank(0.95)} "
        f"p99={nearest_rank(0.99)} max={distinct[-1]}"
    )
    print(
        "winning_table_logs: "
        + " ".join(f"{log}={count}" for log, count in sorted(table_logs.items()))
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
