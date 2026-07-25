#!/usr/bin/env python3
"""Validate and summarize EXP-0074-style focused A/B results."""

from __future__ import annotations

import csv
import statistics
import sys
from collections import defaultdict
from pathlib import Path


STABLE_FIELDS = (
    "encoded_bytes",
    "ratio",
    "encoded_stream_mbps",
    "y_psnr",
    "cb_psnr",
    "cr_psnr",
    "y_block_ssim",
    "max_error",
)


def main() -> int:
    if len(sys.argv) != 2:
        raise SystemExit(f"usage: {Path(sys.argv[0]).name} RESULTS.tsv")
    path = Path(sys.argv[1])
    with path.open(newline="") as source:
        rows = list(csv.DictReader(source, delimiter="\t"))
    groups: dict[tuple[str, str, str], list[dict[str, str]]] = defaultdict(list)
    for row in rows:
        groups[(row["variant"], row["quality"], row["threads"])].append(row)

    expected = {
        (variant, quality, threads)
        for variant in ("baseline", "candidate")
        for quality in ("90", "100")
        for threads in ("1", "4")
    }
    if set(groups) != expected:
        raise SystemExit(f"unexpected result cells: {sorted(set(groups) ^ expected)}")

    print(
        "variant\tquality\tthreads\tencoded_bytes\tratio\tencode_mpps\t"
        "decode_mpps\tencoded_stream_mbps\ty_psnr\ty_block_ssim\tmax_error"
    )
    for key in sorted(groups, key=lambda item: (int(item[1]), int(item[2]), item[0])):
        group = groups[key]
        trials = sorted(int(row["trial"]) for row in group)
        if trials != [1, 2, 3, 4, 5, 6]:
            raise SystemExit(f"{key}: expected trials 1..6, got {trials}")
        for field in STABLE_FIELDS:
            values = {row[field] for row in group}
            if len(values) != 1:
                raise SystemExit(f"{key}: unstable {field}: {sorted(values)}")
        representative = group[0]
        encode = statistics.median(float(row["encode_mpps"]) for row in group)
        decode = statistics.median(float(row["decode_mpps"]) for row in group)
        print(
            "\t".join(
                (
                    key[0],
                    key[1],
                    key[2],
                    representative["encoded_bytes"],
                    representative["ratio"],
                    f"{encode:.3f}",
                    f"{decode:.3f}",
                    representative["encoded_stream_mbps"],
                    representative["y_psnr"],
                    representative["y_block_ssim"],
                    representative["max_error"],
                )
            )
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
