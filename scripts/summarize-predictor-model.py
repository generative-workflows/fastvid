#!/usr/bin/env python3
"""Summarize EXP-0047 exact-byte and reconstruction-error rows."""

import csv
import sys
from collections import Counter, defaultdict


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


def delta(candidate: int, baseline: int) -> float:
    return candidate / baseline - 1


def summarize(rows: list[dict[str, str]]) -> tuple[int, int, int, int]:
    return (
        sum(int(row["current_bytes"]) for row in rows),
        sum(int(row["oracle_bytes"]) for row in rows),
        sum(int(row["current_sse"]) for row in rows),
        sum(int(row["oracle_sse"]) for row in rows),
    )


def format_sse(candidate: int, baseline: int) -> str:
    if baseline == 0:
        return "exact" if candidate == 0 else "REGRESSED_FROM_EXACT"
    return f"{delta(candidate, baseline):+.2%}"


def main() -> None:
    with open(sys.argv[1], encoding="utf-8", newline="") as source:
        rows = list(csv.DictReader(source, delimiter="\t"))
    if not rows:
        raise SystemExit("no model rows")

    current, oracle, current_sse, oracle_sse = summarize(rows)
    frames = {
        (row["sample"], row["frame"], row["bit_depth"], row["quality"], row["gop"])
        for row in rows
    }
    overhead = len(frames) * HEADER_BYTES + len(rows) * DIRECTORY_ENTRY_BYTES
    complete_current = current + overhead
    complete_oracle = oracle + overhead
    wins = sum(int(row["oracle_bytes"]) < int(row["current_bytes"]) for row in rows)
    changed = sum(row["oracle_mode"] != row["current_mode"] for row in rows)
    modes = Counter(row["oracle_mode"] for row in rows)
    savings_by_mode = Counter()
    for row in rows:
        savings_by_mode[row["oracle_mode"]] += int(row["current_bytes"]) - int(
            row["oracle_bytes"]
        )

    print(f"tiles={len(rows)} frames={len(frames)} overhead={overhead}")
    print(
        f"payload current={current} oracle={oracle} "
        f"delta={delta(oracle, current):+.2%}"
    )
    print(
        f"complete_stream current={complete_current} oracle={complete_oracle} "
        f"delta={delta(complete_oracle, complete_current):+.2%}"
    )
    print(
        f"sse current={current_sse} oracle={oracle_sse} "
        f"delta={format_sse(oracle_sse, current_sse)}"
    )
    print(
        f"winning_tiles={wins}/{len(rows)} ({wins / len(rows):.2%}) "
        f"changed_modes={changed}/{len(rows)} ({changed / len(rows):.2%})"
    )
    print(
        "oracle_modes: "
        + " ".join(f"{mode}={count}" for mode, count in sorted(modes.items()))
    )
    print(
        "payload_savings_by_oracle_mode: "
        + " ".join(
            f"{mode}={saving} ({saving / (current - oracle):.2%})"
            for mode, saving in sorted(savings_by_mode.items())
        )
    )

    grouped: dict[tuple[str, str], list[dict[str, str]]] = defaultdict(list)
    for row in rows:
        grouped[("category", category(row["sample"]))].append(row)
        grouped[("bit-depth", row["bit_depth"])].append(row)
        grouped[("quality", row["quality"])].append(row)
        grouped[("sample", row["sample"])].append(row)

    for group_type in ["category", "bit-depth", "quality"]:
        print(f"{group_type}:")
        for (kind, name), group_rows in sorted(grouped.items()):
            if kind != group_type:
                continue
            group_current, group_oracle, group_sse, group_oracle_sse = summarize(
                group_rows
            )
            print(
                f"  {name} tiles={len(group_rows)} "
                f"bytes={delta(group_oracle, group_current):+.2%} "
                f"sse={format_sse(group_oracle_sse, group_sse)}"
            )

    category_results = []
    quality_results = []
    category_quality: dict[tuple[str, str], list[dict[str, str]]] = defaultdict(list)
    for row in rows:
        category_quality[(category(row["sample"]), row["quality"])].append(row)
    for (kind, _), group_rows in grouped.items():
        group_current, group_oracle, group_sse, group_oracle_sse = summarize(group_rows)
        if kind == "category":
            category_results.append(delta(group_oracle, group_current))
        elif kind == "quality":
            quality_results.append(
                0.0 if group_sse == 0 and group_oracle_sse == 0 else delta(group_oracle_sse, group_sse)
            )
    category_quality_sse = []
    for group_rows in category_quality.values():
        _, _, group_sse, group_oracle_sse = summarize(group_rows)
        category_quality_sse.append(
            0.0 if group_sse == 0 and group_oracle_sse == 0 else delta(group_oracle_sse, group_sse)
        )
    q100_exact = all(
        int(row["oracle_sse"]) == 0 for row in rows if row["quality"] == "100"
    )
    gate = (
        delta(complete_oracle, complete_current) <= -0.02
        and sum(result <= -0.01 for result in category_results) >= 2
        and max(quality_results) <= 0.01
        and max(category_quality_sse) <= 0.03
        and q100_exact
        and changed >= wins * 0.10
    )
    print(
        f"worst_quality_sse_delta={max(quality_results):+.2%} "
        f"worst_category_quality_sse_delta={max(category_quality_sse):+.2%} "
        f"q100_exact={q100_exact}"
    )
    print(f"prototype_gate={'PASS' if gate else 'FAIL'}")


if __name__ == "__main__":
    main()
