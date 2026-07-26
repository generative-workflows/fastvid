#!/usr/bin/env python3
"""Compare EXP-0111 candidate-only rows with fixed EXP-0110 results."""

import csv
import math
import statistics
import sys


def load(path: str) -> list[dict[str, str]]:
    with open(path, encoding="utf-8", newline="") as source:
        return list(csv.DictReader(source, delimiter="\t"))


def geometric_mean(values: list[float]) -> float:
    return math.exp(statistics.fmean(math.log(value) for value in values))


def main() -> None:
    minimum_encode_ratio = float(sys.argv[3]) if len(sys.argv) > 3 else 1.75
    reference_variant = sys.argv[4] if len(sys.argv) > 4 else "bounded-full-tile"
    candidate_variant = sys.argv[5] if len(sys.argv) > 5 else None
    reference_rows = [
        row
        for row in load(sys.argv[1])
        if row["variant"] == reference_variant and row["quality"] == "90"
    ]
    candidate_rows = [
        row
        for row in load(sys.argv[2])
        if candidate_variant is None or row["variant"] == candidate_variant
    ]
    samples = sorted({row["sample"] for row in candidate_rows})
    encode_ratios = []
    decode_ratios = []
    exact_bytes = True
    for sample in samples:
        references = [row for row in reference_rows if row["sample"] == sample]
        candidates = [row for row in candidate_rows if row["sample"] == sample]
        encoded_bytes = {int(row["encoded_bytes"]) for row in candidates}
        reference_bytes = {int(row["encoded_bytes"]) for row in references}
        exact_bytes &= encoded_bytes == reference_bytes
        encode = statistics.median(float(row["encode_mpps"]) for row in candidates)
        decode = statistics.median(float(row["decode_mpps"]) for row in candidates)
        reference_encode = statistics.median(
            float(row["encode_mpps"]) for row in references
        )
        reference_decode = statistics.median(
            float(row["decode_mpps"]) for row in references
        )
        encode_ratio = encode / reference_encode
        decode_ratio = decode / reference_decode
        encode_ratios.append(encode_ratio)
        decode_ratios.append(decode_ratio)
        print(
            f"{sample}: encode={encode:.3f} MP/s ({encode_ratio:.3f}x), "
            f"decode={decode:.3f} MP/s ({decode_ratio:.3f}x), "
            f"bitrate={float(candidates[0]['encoded_stream_mbps']):.6f} Mb/s"
        )
    geometric_encode = geometric_mean(encode_ratios)
    geometric_decode = geometric_mean(decode_ratios)
    passed = (
        exact_bytes
        and geometric_encode >= minimum_encode_ratio
        and geometric_decode >= 0.95
    )
    print(
        f"geometric ratios: encode={geometric_encode:.4f}x "
        f"decode={geometric_decode:.4f}x exact_bytes={exact_bytes}"
    )
    print(f"gate={'PASS' if passed else 'FAIL'}")


if __name__ == "__main__":
    main()
