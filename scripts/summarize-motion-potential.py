#!/usr/bin/env python3
"""Summarize EXP-0065 per-block motion-potential rows."""

from __future__ import annotations

import argparse
import csv
from collections import defaultdict
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("results")
    parser.add_argument("summary")
    args = parser.parse_args()

    with Path(args.results).open(newline="", encoding="utf-8") as source:
        rows = list(csv.DictReader(source, delimiter="\t"))
    if not rows:
        raise ValueError("motion-potential results are empty")

    grouped: dict[str, list[dict[str, str]]] = defaultdict(list)
    for row in rows:
        grouped[row["sample"]].append(row)
    fields = [
        "sample",
        "blocks",
        "selected_blocks",
        "selected_pct",
        "baseline_bits",
        "candidate_bits",
        "bits_delta_pct",
        "baseline_sad",
        "candidate_sad",
        "sad_delta_pct",
        "search_evaluations",
        "model_ms",
        "model_mpps",
        "distinct_vectors",
    ]
    with Path(args.summary).open("w", newline="", encoding="utf-8") as target:
        writer = csv.DictWriter(
            target, fieldnames=fields, delimiter="\t", lineterminator="\n"
        )
        writer.writeheader()
        for sample in sorted(grouped):
            group = grouped[sample]
            baseline_bits = sum(int(row["baseline_bits"]) for row in group)
            candidate_bits = sum(int(row["candidate_bits"]) for row in group)
            baseline_sad = sum(int(row["baseline_sad"]) for row in group)
            candidate_sad = sum(int(row["candidate_sad"]) for row in group)
            selected = sum(int(row["selected"]) for row in group)
            vectors = {
                (int(row["dx"]), int(row["dy"]))
                for row in group
                if int(row["selected"])
            }
            writer.writerow(
                {
                    "sample": sample,
                    "blocks": len(group),
                    "selected_blocks": selected,
                    "selected_pct": f"{100 * selected / len(group):.3f}",
                    "baseline_bits": baseline_bits,
                    "candidate_bits": candidate_bits,
                    "bits_delta_pct": f"{100 * (candidate_bits / baseline_bits - 1):.3f}",
                    "baseline_sad": baseline_sad,
                    "candidate_sad": candidate_sad,
                    "sad_delta_pct": f"{100 * (candidate_sad / baseline_sad - 1):.3f}"
                    if baseline_sad
                    else "0.000",
                    "search_evaluations": sum(
                        int(row["search_evaluations"]) for row in group
                    ),
                    "model_ms": group[0]["model_ms"],
                    "model_mpps": group[0]["model_mpps"],
                    "distinct_vectors": len(vectors),
                }
            )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
