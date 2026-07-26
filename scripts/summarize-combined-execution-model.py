#!/usr/bin/env python3
"""Summarize EXP-0107 combined 64-row predictor/four-lane entropy model."""

import csv
import sys
from collections import defaultdict


KEY = ("sample", "frame", "bit_depth", "quality", "gop", "tile")
CONTROL_FIELDS = (
    "band64_complete_bytes",
    "band64_sse",
    "band64_max_error",
)


def delta(candidate: int, baseline: int) -> float:
    return candidate / baseline - 1 if baseline else 0.0


def row_key(row: dict[str, str]) -> tuple[str, ...]:
    return tuple(row[field] for field in KEY)


def main() -> None:
    with open(sys.argv[1], encoding="utf-8", newline="") as source:
        rows = list(csv.DictReader(source, delimiter="\t"))
    with open(sys.argv[2], encoding="utf-8", newline="") as source:
        controls = {row_key(row): row for row in csv.DictReader(source, delimiter="\t")}

    control_mismatches = 0
    for row in rows:
        control = controls.get(row_key(row))
        if control is None or any(row[field] != control[field] for field in CONTROL_FIELDS):
            control_mismatches += 1

    grouped: dict[tuple[str, str], list[dict[str, str]]] = defaultdict(list)
    for row in rows:
        grouped[(row["sample"], row["bit_depth"])].append(row)

    baseline = sum(int(row["clamp_bytes"]) for row in rows)
    candidate = sum(int(row["band64_parallel_complete_bytes"]) for row in rows)
    max_predictor_samples = max(int(row["band64_max_samples"]) for row in rows)
    max_entropy_span = max(int(row["band64_max_entropy_span"]) for row in rows)
    sample_results = []
    for (sample, depth), sample_rows in grouped.items():
        sample_baseline = sum(int(row["clamp_bytes"]) for row in sample_rows)
        sample_candidate = sum(
            int(row["band64_parallel_complete_bytes"]) for row in sample_rows
        )
        sample_results.append(
            (delta(sample_candidate, sample_baseline), sample, depth,
             sample_baseline, sample_candidate)
        )

    aggregate_delta = delta(candidate, baseline)
    worst_sample_delta = max(value[0] for value in sample_results)
    passed = (
        max_predictor_samples <= 16384
        and max_entropy_span <= 4096
        and aggregate_delta <= 0.03
        and worst_sample_delta <= 0.05
        and control_mismatches == 0
    )

    print(
        f"tiles={len(rows)} max_predictor_samples={max_predictor_samples} "
        f"max_entropy_span={max_entropy_span}"
    )
    print(
        f"aggregate: baseline={baseline} candidate={candidate} "
        f"delta={aggregate_delta:+.4%}"
    )
    print(f"EXP-0104 control mismatches={control_mismatches}")
    for value, sample, depth, sample_baseline, sample_candidate in sorted(
        sample_results
    ):
        print(
            f"{sample} depth={depth}: baseline={sample_baseline} "
            f"candidate={sample_candidate} delta={value:+.4%}"
        )
    print(f"gate={'PASS' if passed else 'FAIL'}")


if __name__ == "__main__":
    main()
