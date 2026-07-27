#!/usr/bin/env python3
import csv
import math
import sys
from collections import Counter, defaultdict
from pathlib import Path


if len(sys.argv) != 4:
    raise SystemExit("usage: summarize-quality-sweep.py INPUT.tsv OUTPUT.md OUTPUT.tsv")

input_path, markdown_path, summary_path = map(Path, sys.argv[1:])
with input_path.open(newline="") as stream:
    rows = list(csv.DictReader(stream, delimiter="\t"))

groups = defaultdict(list)
for row in rows:
    groups[int(row["quality"])].append(row)


def summarize(selected):
    raw = sum(int(row["raw_bytes"]) for row in selected)
    encoded = sum(int(row["encoded_bytes"]) for row in selected)
    xpsnr = [float(row["xpsnr_y_db"]) for row in selected]
    return {
        "samples": len(selected),
        "step": int(selected[0]["quant_step"]),
        "ratio": raw / encoded,
        "minimum_xpsnr": min(xpsnr),
        "ratio_pass": sum(row["compression_gt_15x"] == "true" for row in selected),
        "xpsnr_pass": sum(row["xpsnr_gt_50db"] == "true" for row in selected),
        "exact": sum(row["exact"] == "true" for row in selected),
    }


summaries = []
for quality, quality_rows in sorted(groups.items()):
    corpus = summarize(quality_rows)
    corpus.update({"scope": "corpus", "quality": quality})
    summaries.append(corpus)
    hd_rows = [row for row in quality_rows if (int(row["width"]), int(row["height"])) == (1920, 1080)]
    hd = summarize(hd_rows)
    hd.update({"scope": "1080p", "quality": quality})
    summaries.append(hd)


def adaptive_oracle(candidates, scope):
    by_sample = defaultdict(list)
    for row in candidates:
        by_sample[row["sample"]].append(row)
    selected = []
    for sample_rows in by_sample.values():
        eligible = [row for row in sample_rows if row["xpsnr_gt_50db"] == "true"]
        selected.append(max(eligible, key=lambda row: int(row["quant_step"])))
    result = summarize(selected)
    result.update({"scope": scope, "quality": "adaptive", "step": "mixed"})
    return result, Counter(int(row["quality"]) for row in selected)


adaptive_corpus, adaptive_corpus_counts = adaptive_oracle(rows, "corpus-oracle")
adaptive_hd, adaptive_hd_counts = adaptive_oracle(
    [row for row in rows if (int(row["width"]), int(row["height"])) == (1920, 1080)],
    "1080p-oracle",
)
summaries.extend((adaptive_corpus, adaptive_hd))

fields = ["scope", "quality", "step", "samples", "ratio", "minimum_xpsnr", "ratio_pass", "xpsnr_pass", "exact", "simultaneous_aggregate_pass"]
summary_path.parent.mkdir(parents=True, exist_ok=True)
with summary_path.open("w", newline="") as stream:
    writer = csv.DictWriter(stream, fields, delimiter="\t")
    writer.writeheader()
    for row in summaries:
        aggregate_pass = row["ratio"] > 15 and row["minimum_xpsnr"] > 50
        writer.writerow({
            **row,
            "ratio": f"{row['ratio']:.9f}",
            "minimum_xpsnr": "inf" if math.isinf(row["minimum_xpsnr"]) else f"{row['minimum_xpsnr']:.6f}",
            "simultaneous_aggregate_pass": str(aggregate_pass).lower(),
        })

lines = [
    "# Version-5 full-corpus rate-distortion sweep",
    "",
    "Weighted compression uses total raw bytes divided by total encoded bytes. The quality gate is the minimum luma XPSNR across samples, so an aggregate average cannot hide a failing input.",
    "",
    "| Scope | Q | Step | Samples | Compression | >15x samples | Min Y XPSNR | >50 dB samples | Simultaneous aggregate pass |",
    "|---|---:|---:|---:|---:|---:|---:|---:|:---:|",
]
for row in summaries:
    minimum = "inf" if math.isinf(row["minimum_xpsnr"]) else f"{row['minimum_xpsnr']:.4f} dB"
    aggregate_pass = row["ratio"] > 15 and row["minimum_xpsnr"] > 50
    lines.append(
        f"| {row['scope']} | {row['quality']} | {row['step']} | {row['samples']} | {row['ratio']:.6f}x | "
        f"{row['ratio_pass']}/{row['samples']} | {minimum} | {row['xpsnr_pass']}/{row['samples']} | "
        f"{'yes' if aggregate_pass else 'no'} |"
    )
lines += [
    "",
    "The content-adaptive oracle chooses the coarsest tested step that keeps each individual sample above 50 dB. It uses "
    + ", ".join(f"q{quality} for {count}" for quality, count in sorted(adaptive_corpus_counts.items()))
    + " corpus samples; the 1080p selection uses "
    + ", ".join(f"q{quality} for {count}" for quality, count in sorted(adaptive_hd_counts.items()))
    + ".",
    "",
    "Q100 is the exactness control. The speed targets are evaluated separately; this table locates the deterministic rate-quality boundary used to choose the next compression experiment.",
    "",
]
markdown_path.parent.mkdir(parents=True, exist_ok=True)
markdown_path.write_text("\n".join(lines))
