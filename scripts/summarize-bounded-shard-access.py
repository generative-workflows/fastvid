#!/usr/bin/env python3
"""Summarize EXP-0108 independent tile access throughput."""

import csv
import sys


def main() -> None:
    with open(sys.argv[1], encoding="utf-8", newline="") as source:
        rows = {row["variant"]: row for row in csv.DictReader(source, delimiter="\t")}
    baseline = float(rows["baseline"]["tile_sample_mpps"])
    candidate = float(rows["bounded-shard"]["tile_sample_mpps"])
    print(
        f"tile access: baseline={baseline:.3f} candidate={candidate:.3f} "
        f"delta={candidate / baseline - 1:+.2%}"
    )


if __name__ == "__main__":
    main()
