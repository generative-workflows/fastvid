#!/usr/bin/env python3
"""Summarize EXP-0037 without third-party dependencies."""

import csv
import itertools
import math
import statistics
import sys


def percentile(values: list[float], fraction: float) -> float:
    ordered = sorted(values)
    index = (len(ordered) - 1) * fraction
    lower = math.floor(index)
    upper = math.ceil(index)
    if lower == upper:
        return ordered[lower]
    return ordered[lower] * (upper - index) + ordered[upper] * (index - lower)


def ranks(values: list[float]) -> list[float]:
    order = sorted(range(len(values)), key=values.__getitem__)
    result = [0.0] * len(values)
    start = 0
    while start < len(order):
        end = start + 1
        while end < len(order) and values[order[end]] == values[order[start]]:
            end += 1
        average = (start + end - 1) / 2 + 1
        for position in range(start, end):
            result[order[position]] = average
        start = end
    return result


def correlation(left: list[float], right: list[float]) -> float:
    left_mean = statistics.fmean(left)
    right_mean = statistics.fmean(right)
    numerator = sum(
        (left_value - left_mean) * (right_value - right_mean)
        for left_value, right_value in zip(left, right, strict=True)
    )
    left_energy = sum((value - left_mean) ** 2 for value in left)
    right_energy = sum((value - right_mean) ** 2 for value in right)
    return numerator / math.sqrt(left_energy * right_energy)


def reversals(rows: list[dict[str, str]], sampled_key: str, tolerance: float) -> int:
    total = 0
    by_sample: dict[str, list[dict[str, str]]] = {}
    for row in rows:
        by_sample.setdefault(row["sample"], []).append(row)
    for sample_rows in by_sample.values():
        for left, right in itertools.combinations(sample_rows, 2):
            exact_delta = float(left["exact_ssim"]) - float(right["exact_ssim"])
            sampled_delta = float(left[sampled_key]) - float(right[sampled_key])
            if abs(exact_delta) > tolerance and exact_delta * sampled_delta < 0:
                total += 1
    return total


def main() -> None:
    with open(sys.argv[1], encoding="utf-8", newline="") as source:
        rows = list(csv.DictReader(source, delimiter="\t"))
    exact = [float(row["exact_ssim"]) for row in rows]
    print(f"rows: {len(rows)}")
    for stride, error_limit, rho_limit, speed_limit in [
        (2, 0.0005, 0.999, 2.0),
        (5, 0.001, 0.995, 8.0),
    ]:
        errors = [float(row[f"sample{stride}_abs_error"]) for row in rows]
        sampled = [float(row[f"sample{stride}_ssim"]) for row in rows]
        speedups = [
            float(row[f"sample{stride}_speedup"])
            for row in rows
            if int(row["width"]) >= 1920 and int(row["height"]) >= 1080
        ]
        rho = correlation(ranks(exact), ranks(sampled))
        ordering_reversals = reversals(
            rows, f"sample{stride}_ssim", error_limit
        )
        accepted = (
            max(errors) <= error_limit
            and rho >= rho_limit
            and ordering_reversals == 0
            and statistics.median(speedups) >= speed_limit
        )
        worst = max(rows, key=lambda row: float(row[f"sample{stride}_abs_error"]))
        print(
            f"stride {stride}: max_error={max(errors):.9f} "
            f"p50={percentile(errors, 0.5):.9f} "
            f"p95={percentile(errors, 0.95):.9f} rho={rho:.9f} "
            f"reversals={ordering_reversals} "
            f"median_1080p_speedup={statistics.median(speedups):.3f}x "
            f"gate={'PASS' if accepted else 'FAIL'}"
        )
        print(
            f"  worst={worst['sample']} q{worst['quality']} "
            f"exact={float(worst['exact_ssim']):.9f} "
            f"sampled={float(worst[f'sample{stride}_ssim']):.9f}"
        )


if __name__ == "__main__":
    main()
