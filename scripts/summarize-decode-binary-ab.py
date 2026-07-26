#!/usr/bin/env python3
"""Summarize a balanced isolated-decode arbitrary-binary comparison."""

import csv
import statistics
import sys


def main() -> None:
    if len(sys.argv) != 5:
        raise SystemExit(
            "usage: summarize-decode-binary-ab.py "
            "RESULTS REFERENCE_LABEL CANDIDATE_LABEL MIN_DECODE_RATIO"
        )
    with open(sys.argv[1], encoding="utf-8", newline="") as source:
        rows = list(csv.DictReader(source, delimiter="\t"))
    reference_label = sys.argv[2]
    candidate_label = sys.argv[3]
    minimum_ratio = float(sys.argv[4])
    reference = [row for row in rows if row["variant"] == reference_label]
    candidate = [row for row in rows if row["variant"] == candidate_label]
    if not reference or len(reference) != len(candidate):
        raise SystemExit("balanced variants are missing or have unequal trial counts")

    invariant_fields = (
        "input",
        "encoded_bytes",
        "threads",
        "repetitions",
        "bit_depth",
        "luma_pixels",
    )
    invariant = all(
        {row[field] for row in rows} == {reference[0][field]}
        for field in invariant_fields
    )
    reference_mpps = statistics.median(float(row["decode_mpps"]) for row in reference)
    candidate_mpps = statistics.median(float(row["decode_mpps"]) for row in candidate)
    ratio = candidate_mpps / reference_mpps
    passed = invariant and ratio >= minimum_ratio
    print(
        f"reference={reference_mpps:.3f} MP/s "
        f"candidate={candidate_mpps:.3f} MP/s ratio={ratio:.4f}x "
        f"invariant={invariant}"
    )
    print(f"gate={'PASS' if passed else 'FAIL'}")


if __name__ == "__main__":
    main()
