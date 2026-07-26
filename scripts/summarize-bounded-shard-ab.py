#!/usr/bin/env python3
"""Summarize EXP-0108 high-bit bounded-shard prototype."""

import csv
import math
import statistics
import sys
from collections import defaultdict


def delta(candidate: float, baseline: float) -> float:
    return candidate / baseline - 1


def geometric_mean(values: list[float]) -> float:
    return math.exp(statistics.fmean(math.log(value) for value in values))


def main() -> None:
    with open(sys.argv[1], encoding="utf-8", newline="") as source:
        rows = list(csv.DictReader(source, delimiter="\t"))

    grouped: dict[tuple[str, str, str], list[dict[str, str]]] = defaultdict(list)
    for row in rows:
        grouped[(row["sample"], row["bit_depth"], row["quality"])].append(row)

    selected: dict[tuple[str, str, str, str], dict[str, float]] = {}
    for (sample, depth, quality), group_rows in grouped.items():
        for variant in ("baseline", "bounded-shard"):
            variant_rows = [row for row in group_rows if row["variant"] == variant]
            selected[(sample, depth, quality, variant)] = {
                field: statistics.median(float(row[field]) for row in variant_rows)
                for field in (
                    "encoded_bytes",
                    "encode_mpps",
                    "decode_mpps",
                    "encode_raw_mb_s",
                    "decode_raw_mb_s",
                    "encoded_stream_mbps",
                    "y_psnr",
                    "y_block_ssim",
                    "max_error",
                )
            }

    q90_results = []
    for sample, depth, quality in sorted(grouped):
        if quality != "90":
            continue
        baseline = selected[(sample, depth, quality, "baseline")]
        candidate = selected[(sample, depth, quality, "bounded-shard")]
        q90_results.append(
            (
                delta(candidate["encoded_bytes"], baseline["encoded_bytes"]),
                sample,
                depth,
                baseline,
                candidate,
            )
        )

    baseline_bytes = sum(result[3]["encoded_bytes"] for result in q90_results)
    candidate_bytes = sum(result[4]["encoded_bytes"] for result in q90_results)
    aggregate_delta = delta(candidate_bytes, baseline_bytes)
    worst_sample_delta = max(result[0] for result in q90_results)
    encode_ratio = geometric_mean(
        [result[4]["encode_mpps"] / result[3]["encode_mpps"] for result in q90_results]
    )
    decode_ratio = geometric_mean(
        [result[4]["decode_mpps"] / result[3]["decode_mpps"] for result in q90_results]
    )
    q100_exact = all(
        values["max_error"] == 0
        for key, values in selected.items()
        if key[2] == "100" and key[3] == "bounded-shard"
    )
    passed = aggregate_delta <= 0.03 and worst_sample_delta <= 0.05 and q100_exact

    print(
        f"q90 aggregate: baseline={baseline_bytes:.0f} candidate={candidate_bytes:.0f} "
        f"delta={aggregate_delta:+.4%}"
    )
    print(
        f"q90 geometric throughput ratio: encode={encode_ratio:.4f}x "
        f"decode={decode_ratio:.4f}x"
    )
    for value, sample, depth, baseline, candidate in q90_results:
        print(
            f"{sample} depth={depth}: bytes={value:+.4%} "
            f"encode={candidate['encode_mpps']:.3f} MP/s "
            f"decode={candidate['decode_mpps']:.3f} MP/s "
            f"bitrate={candidate['encoded_stream_mbps']:.6f} Mb/s "
            f"PSNR={candidate['y_psnr']:.6f} "
            f"SSIM={candidate['y_block_ssim']:.8f} "
            f"max_error={candidate['max_error']:.0f}"
        )
    print(f"q100_exact={q100_exact}")
    print(f"gate={'PASS' if passed else 'FAIL'}")


if __name__ == "__main__":
    main()
