#!/usr/bin/env python3
"""Summarize EXP-0056 charged causal-context order-0 models."""

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
    if sample.startswith(("ui-", "procedural-", "resolution-")):
        return "synthetic-ui"
    raise ValueError(f"unclassified sample: {sample}")


def summarize(rows: list[dict[str, str]]) -> dict[str, int]:
    frames = {
        (row["sample"], row["frame"], row["quality"], row["gop"]) for row in rows
    }
    overhead = len(frames) * HEADER_BYTES + len(rows) * DIRECTORY_ENTRY_BYTES
    baseline_payload = sum(int(row["order0_complete_bytes"]) for row in rows)
    context_payload = 0
    wins = 0
    table_bytes = 0
    control_bytes = 0
    for row in rows:
        baseline = int(row["order0_complete_bytes"])
        context = int(row["context_order0_complete_bytes"])
        if row["context_order0_supported"] == "true" and context < baseline:
            context_payload += context
            wins += 1
            table_bytes += int(row["context_order0_table_bytes"])
            control_bytes += int(row["context_order0_control_bytes"])
        else:
            context_payload += baseline
    return {
        "tiles": len(rows),
        "frames": len(frames),
        "baseline_payload": baseline_payload,
        "context_payload": context_payload,
        "baseline_complete": baseline_payload + overhead,
        "context_complete": context_payload + overhead,
        "wins": wins,
        "table": table_bytes,
        "control": control_bytes,
    }


def main() -> int:
    if len(sys.argv) != 2:
        raise SystemExit(f"usage: {Path(sys.argv[0]).name} RESULTS.tsv")
    with Path(sys.argv[1]).open(newline="", encoding="utf-8") as source:
        rows = list(csv.DictReader(source, delimiter="\t"))
    if not rows:
        raise SystemExit("no model rows")

    print(
        "group\ttiles\tframes\torder0_complete\tcontext_oracle_complete"
        "\tdelta_pct\twinning_tiles\twinning_table_bytes\twinning_control_bytes"
    )
    sample_groups: dict[str, list[dict[str, str]]] = defaultdict(list)
    for row in rows:
        sample_groups[row["sample"]].append(row)
    for name, group in [*sorted(sample_groups.items()), ("TOTAL", rows)]:
        result = summarize(group)
        print(
            f"{name}\t{result['tiles']}\t{result['frames']}"
            f"\t{result['baseline_complete']}\t{result['context_complete']}"
            f"\t{percent(result['context_complete'], result['baseline_complete']):.3f}"
            f"\t{result['wins']}\t{result['table']}\t{result['control']}"
        )

    for field, key in [
        ("category", lambda row: category(row["sample"])),
        ("quality", lambda row: row["quality"]),
        ("prediction", lambda row: row["prediction"]),
        ("plane", lambda row: row["plane"]),
    ]:
        groups: dict[str, list[dict[str, str]]] = defaultdict(list)
        for row in rows:
            groups[str(key(row))].append(row)
        print(f"{field}:")
        for name, group in sorted(groups.items()):
            result = summarize(group)
            print(
                f"  {name}: tiles={result['tiles']} "
                f"complete={percent(result['context_complete'], result['baseline_complete']):+.3f}% "
                f"wins={result['wins']}"
            )

    winners = [
        row
        for row in rows
        if row["context_order0_supported"] == "true"
        and int(row["context_order0_complete_bytes"])
        < int(row["order0_complete_bytes"])
    ]
    choices = Counter(
        (row["context_order0_contexts"], row["context_order0_threshold"])
        for row in winners
    )
    print(
        "winning_contexts: "
        + " ".join(
            f"{contexts}ctx/t{threshold}={count}"
            for (contexts, threshold), count in sorted(choices.items())
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
