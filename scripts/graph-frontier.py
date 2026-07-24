#!/usr/bin/env python3
"""Validate frontier measurements and render compression/throughput Pareto plots."""

from __future__ import annotations

import argparse
import csv
import hashlib
import html
import json
import math
import statistics
from collections import defaultdict
from pathlib import Path


def geometric_mean(values: list[float]) -> float:
    if not values or any(value <= 0 for value in values):
        raise ValueError("geometric mean needs positive values")
    return math.exp(sum(math.log(value) for value in values) / len(values))


def pareto(points: list[dict[str, float]], speed: str) -> list[dict[str, float]]:
    return sorted(
        [
            point
            for point in points
            if not any(
                other["ratio"] >= point["ratio"]
                and other[speed] >= point[speed]
                and (
                    other["ratio"] > point["ratio"]
                    or other[speed] > point[speed]
                )
                for other in points
                if other is not point
            )
        ],
        key=lambda point: point["ratio"],
    )


def load_points(
    manifest_path: Path, results_path: Path
) -> tuple[dict, list[dict[str, float | str]], list[dict[str, str]]]:
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    slots = {slot["id"]: slot for slot in manifest["slots"]}
    with results_path.open(newline="", encoding="utf-8") as source:
        rows = list(csv.DictReader(source, delimiter="\t"))
    if not rows:
        raise ValueError("frontier results are empty")

    grouped: dict[tuple[str, str], list[dict[str, str]]] = defaultdict(list)
    for row in rows:
        if row["slot"] not in slots or slots[row["slot"]]["state"] == "vacant":
            raise ValueError(f"unknown or vacant measured slot: {row['slot']}")
        grouped[(row["slot"], row["case"])].append(row)

    active = [slot for slot in manifest["slots"] if slot["state"] != "vacant"]
    cases = sorted({row["case"] for row in rows})
    expected_trials = int(manifest["protocol"]["trials"])
    points: list[dict[str, float | str]] = []
    summary: list[dict[str, str]] = []
    for slot in active:
        medians: list[dict[str, float]] = []
        total_bytes = 0
        for case in cases:
            group = grouped.get((slot["id"], case), [])
            trials = {int(row["trial"]) for row in group}
            if trials != set(range(1, expected_trials + 1)):
                raise ValueError(
                    f"{slot['id']}/{case} has trials {sorted(trials)}, "
                    f"expected 1..{expected_trials}"
                )
            encoded = {int(row["encoded_bytes"]) for row in group}
            if len(encoded) != 1:
                raise ValueError(f"{slot['id']}/{case} encoded size changed across trials")
            total_bytes += encoded.pop()
            medians.append(
                {
                    "ratio": statistics.median(float(row["ratio"]) for row in group),
                    "encode_mpps": statistics.median(
                        float(row["encode_mpps"]) for row in group
                    ),
                    "decode_mpps": statistics.median(
                        float(row["decode_mpps"]) for row in group
                    ),
                    "bitrate_mbps": statistics.median(
                        float(row["encoded_stream_mbps"]) for row in group
                    ),
                }
            )
        point = {
            "id": slot["id"],
            "label": slot["label"],
            "ratio": geometric_mean([row["ratio"] for row in medians]),
            "encode_mpps": geometric_mean([row["encode_mpps"] for row in medians]),
            "decode_mpps": geometric_mean([row["decode_mpps"] for row in medians]),
            "bitrate_mbps": geometric_mean([row["bitrate_mbps"] for row in medians]),
            "encoded_bytes": float(total_bytes),
        }
        points.append(point)
        summary.append(
            {
                "slot": slot["id"],
                "label": slot["label"],
                "state": slot["state"],
                "ratio": f"{point['ratio']:.6f}",
                "encode_mpps": f"{point['encode_mpps']:.6f}",
                "decode_mpps": f"{point['decode_mpps']:.6f}",
                "encoded_stream_mbps": f"{point['bitrate_mbps']:.6f}",
                "encoded_bytes": str(total_bytes),
            }
        )
    for slot in manifest["slots"]:
        if slot["state"] == "vacant":
            summary.append(
                {
                    "slot": slot["id"],
                    "label": slot["label"],
                    "state": "vacant",
                    "ratio": "",
                    "encode_mpps": "",
                    "decode_mpps": "",
                    "encoded_stream_mbps": "",
                    "encoded_bytes": "",
                }
            )
    return manifest, points, summary


def render_svg(manifest: dict, points: list[dict[str, float | str]]) -> str:
    width, height = 1160, 620
    colors = {
        "balanced": "#2563eb",
        "practical-compression": "#059669",
        "maximum-compression": "#dc2626",
        "speed": "#7c3aed",
    }

    def panel(x0: float, y0: float, w: float, h: float, speed: str, title: str) -> str:
        numeric = points  # type: ignore[assignment]
        xs = [float(point["ratio"]) for point in numeric]
        ys = [float(point[speed]) for point in numeric]
        xmin, xmax = min(xs) * 0.94, max(xs) * 1.06
        ymin, ymax = min(ys) * 0.88, max(ys) * 1.12

        def sx(value: float) -> float:
            return x0 + (value - xmin) / (xmax - xmin) * w

        def sy(value: float) -> float:
            return y0 + h - (value - ymin) / (ymax - ymin) * h

        out = [
            f'<text x="{x0}" y="{y0 - 24}" class="panel-title">{html.escape(title)}</text>',
            f'<rect x="{x0}" y="{y0}" width="{w}" height="{h}" class="plot"/>',
        ]
        for tick in range(5):
            xv = xmin + (xmax - xmin) * tick / 4
            yv = ymin + (ymax - ymin) * tick / 4
            out.append(
                f'<line x1="{sx(xv):.2f}" y1="{y0}" x2="{sx(xv):.2f}" '
                f'y2="{y0 + h}" class="grid"/>'
            )
            out.append(
                f'<text x="{sx(xv):.2f}" y="{y0 + h + 24}" class="tick" '
                f'text-anchor="middle">{xv:.1f}×</text>'
            )
            out.append(
                f'<line x1="{x0}" y1="{sy(yv):.2f}" x2="{x0 + w}" '
                f'y2="{sy(yv):.2f}" class="grid"/>'
            )
            out.append(
                f'<text x="{x0 - 10}" y="{sy(yv) + 4:.2f}" class="tick" '
                f'text-anchor="end">{yv:.1f}</text>'
            )
        front = pareto(numeric, speed)  # type: ignore[arg-type]
        if len(front) > 1:
            coords = " ".join(
                f"{sx(float(point['ratio'])):.2f},{sy(float(point[speed])):.2f}"
                for point in front
            )
            out.append(f'<polyline points="{coords}" class="frontier"/>')
        for index, point in enumerate(numeric):
            px, py = sx(float(point["ratio"])), sy(float(point[speed]))
            color = colors.get(str(point["id"]), "#475569")
            label_y = py - 13 if index % 2 == 0 else py + 25
            out.extend(
                [
                    f'<circle cx="{px:.2f}" cy="{py:.2f}" r="7" fill="{color}"/>',
                    f'<text x="{px + 10:.2f}" y="{label_y:.2f}" class="label">'
                    f'{html.escape(str(point["label"]))}</text>',
                ]
            )
        out.append(
            f'<text x="{x0 + w / 2}" y="{y0 + h + 56}" class="axis" '
            'text-anchor="middle">Compression ratio (higher is better)</text>'
        )
        out.append(
            f'<text x="{x0 - 58}" y="{y0 + h / 2}" class="axis" '
            f'text-anchor="middle" transform="rotate(-90 {x0 - 58} {y0 + h / 2})">'
            'Throughput (luma MP/s, higher is better)</text>'
        )
        return "".join(out)

    protocol = manifest["protocol"]
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
        f'viewBox="0 0 {width} {height}">'
        "<style>"
        "text{font-family:ui-sans-serif,system-ui,sans-serif;fill:#172033}"
        ".title{font-size:24px;font-weight:700}.subtitle{font-size:13px;fill:#526071}"
        ".panel-title{font-size:17px;font-weight:650}.plot{fill:#fbfdff;stroke:#b9c4d0}"
        ".grid{stroke:#dce3ea;stroke-width:1}.tick{font-size:11px;fill:#667587}"
        ".axis{font-size:12px;font-weight:600}.label{font-size:12px;font-weight:600}"
        ".frontier{fill:none;stroke:#111827;stroke-width:2;stroke-dasharray:5 4}"
        "</style>"
        '<rect width="100%" height="100%" fill="#f4f7fa"/>'
        '<text x="42" y="40" class="title">Fastvid codec frontier</text>'
        f'<text x="42" y="64" class="subtitle">{html.escape(protocol["scope"])}; '
        f'{html.escape(protocol["aggregation"])}</text>'
        + panel(94, 120, 455, 390, "encode_mpps", "Encode frontier")
        + panel(674, 120, 455, 390, "decode_mpps", "Decode frontier")
        + '<text x="42" y="602" class="subtitle">Dashed lines connect non-dominated '
        "points within each two-axis view. Quality/settings are identical per case. "
        "The explicit speed slot is omitted while vacant.</text>"
        "</svg>"
    )


def write_summary(path: Path, rows: list[dict[str, str]]) -> None:
    fields = [
        "slot",
        "label",
        "state",
        "ratio",
        "encode_mpps",
        "decode_mpps",
        "encoded_stream_mbps",
        "encoded_bytes",
    ]
    with path.open("w", newline="", encoding="utf-8") as target:
        writer = csv.DictWriter(target, fieldnames=fields, delimiter="\t")
        writer.writeheader()
        writer.writerows(rows)


def self_test() -> None:
    points = [
        {"ratio": 2.0, "speed": 10.0},
        {"ratio": 3.0, "speed": 8.0},
        {"ratio": 1.5, "speed": 9.0},
        {"ratio": 2.5, "speed": 7.0},
    ]
    assert pareto(points, "speed") == points[:2]
    assert abs(geometric_mean([2.0, 8.0]) - 4.0) < 1e-12


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("results", nargs="?")
    parser.add_argument("svg", nargs="?")
    parser.add_argument("--manifest", default="frontier.json")
    parser.add_argument("--summary")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return 0
    if not args.results or not args.svg:
        parser.error("RESULTS and SVG are required unless --self-test is used")
    manifest_path = Path(args.manifest)
    results_path = Path(args.results)
    manifest, points, summary = load_points(manifest_path, results_path)
    Path(args.svg).write_text(render_svg(manifest, points), encoding="utf-8")
    if args.summary:
        write_summary(Path(args.summary), summary)
    digest = hashlib.sha256(results_path.read_bytes()).hexdigest()
    print(f"results_sha256={digest}")
    for point in points:
        print(
            f"{point['id']}: ratio={float(point['ratio']):.3f}x "
            f"encode={float(point['encode_mpps']):.3f} MP/s "
            f"decode={float(point['decode_mpps']):.3f} MP/s"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
