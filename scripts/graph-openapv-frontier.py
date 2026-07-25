#!/usr/bin/env python3
"""Validate and graph the EXP-0073 matched 10-bit external reference."""

from __future__ import annotations

import argparse
import csv
import hashlib
import html
import math
import statistics
from collections import defaultdict
from pathlib import Path


NUMERIC_FIELDS = [
    "encoded_bytes",
    "ratio",
    "bits_per_luma_pixel",
    "encode_ms",
    "decode_ms",
    "encode_mpps",
    "decode_mpps",
    "encoded_stream_mbps",
    "y_psnr",
    "y_block_ssim",
    "max_error",
]


def aggregate(rows: list[dict[str, str]]) -> dict[str, str | float]:
    trials = sorted(int(row["trial"]) for row in rows)
    if trials != list(range(1, len(rows) + 1)):
        raise ValueError(f"non-contiguous trials: {trials}")
    stable = [
        "encoded_bytes",
        "ratio",
        "bits_per_luma_pixel",
        "y_psnr",
        "cb_psnr",
        "cr_psnr",
        "y_block_ssim",
        "max_error",
    ]
    for field in stable:
        if len({row[field] for row in rows}) != 1:
            raise ValueError(f"{field} changed across trials")
    result: dict[str, str | float] = {
        field: rows[0][field]
        for field in [
            "codec",
            "slot",
            "label",
            "preset",
            "control",
            "threads",
        ]
    }
    for field in NUMERIC_FIELDS:
        result[field] = statistics.median(float(row[field]) for row in rows)
    return result


def load(path: Path, expected_trials: int) -> list[dict[str, str | float]]:
    with path.open(newline="", encoding="utf-8") as source:
        rows = list(csv.DictReader(source, delimiter="\t"))
    if not rows:
        raise ValueError("external-reference results are empty")
    groups: dict[tuple[str, ...], list[dict[str, str]]] = defaultdict(list)
    for row in rows:
        key = (
            row["codec"],
            row["slot"],
            row["preset"],
            row["control"],
            row["threads"],
        )
        groups[key].append(row)
    expected_groups = {
        ("fastvid", slot, "frontier", quality, str(threads))
        for slot in ["speed", "practical-compression", "maximum-compression"]
        for quality in ["q90", "q100"]
        for threads in [1, 4]
    } | {
        ("openapv", "external", preset, f"qp{qp}", str(threads))
        for preset in ["medium", "fastest"]
        for qp in [0, 20, 21, 22, 23, 24]
        for threads in [1, 4]
    }
    if set(groups) != expected_groups:
        missing = sorted(expected_groups - set(groups))
        extra = sorted(set(groups) - expected_groups)
        raise ValueError(f"matrix mismatch; missing={missing}, extra={extra}")
    trial_counts = {len(group) for group in groups.values()}
    if trial_counts != {expected_trials}:
        raise ValueError(
            f"trial counts {sorted(trial_counts)}, expected {expected_trials}"
        )
    return [aggregate(group) for _, group in sorted(groups.items())]


def control_number(control: str) -> int:
    return int(control.removeprefix("q").removeprefix("p"))


def select(
    rows: list[dict[str, str | float]], threads: int, high_fidelity: bool
) -> tuple[list[dict[str, str | float]], float]:
    thread_rows = [row for row in rows if int(str(row["threads"])) == threads]
    practical = next(
        row
        for row in thread_rows
        if row["codec"] == "fastvid"
        and row["slot"] == "practical-compression"
        and row["control"] == "q90"
    )
    target = float(practical["y_psnr"])
    if high_fidelity:
        selected = [
            row
            for row in thread_rows
            if (row["codec"] == "fastvid" and row["control"] == "q100")
            or (row["codec"] == "openapv" and row["control"] == "qp0")
        ]
    else:
        selected = [
            row
            for row in thread_rows
            if row["codec"] == "fastvid" and row["control"] == "q90"
        ]
        for preset in ["medium", "fastest"]:
            candidates = [
                row
                for row in thread_rows
                if row["codec"] == "openapv"
                and row["preset"] == preset
                and row["control"] != "qp0"
            ]
            selected.append(
                min(
                    candidates,
                    key=lambda row: (
                        abs(float(row["y_psnr"]) - target),
                        control_number(str(row["control"])),
                    ),
                )
            )
    order = {
        "speed": 0,
        "practical-compression": 1,
        "maximum-compression": 2,
        "medium": 3,
        "fastest": 4,
    }
    selected.sort(
        key=lambda row: order.get(
            str(row["slot"] if row["codec"] == "fastvid" else row["preset"]), 9
        )
    )
    return selected, target


def render_svg(rows: list[dict[str, str | float]], target: float) -> str:
    width, height = 1160, 650
    colors = {
        "speed": "#7c3aed",
        "practical-compression": "#059669",
        "maximum-compression": "#dc2626",
        "medium": "#334155",
        "fastest": "#ea580c",
    }

    def panel(x0: float, speed_field: str, title: str) -> str:
        y0, plot_width, plot_height = 130.0, 455.0, 390.0
        xs = [float(row["ratio"]) for row in rows]
        ys = [float(row[speed_field]) for row in rows]
        xmin, xmax = min(xs) * 0.94, max(xs) * 1.06
        ymin, ymax = min(ys) * 0.85, max(ys) * 1.15

        def sx(value: float) -> float:
            return x0 + (value - xmin) / (xmax - xmin) * plot_width

        def sy(value: float) -> float:
            return y0 + plot_height - (value - ymin) / (ymax - ymin) * plot_height

        output = [
            f'<text x="{x0}" y="{y0 - 24}" class="panel-title">{title}</text>',
            f'<rect x="{x0}" y="{y0}" width="{plot_width}" '
            f'height="{plot_height}" class="plot"/>',
        ]
        for tick in range(5):
            xv = xmin + (xmax - xmin) * tick / 4
            yv = ymin + (ymax - ymin) * tick / 4
            output.extend(
                [
                    f'<line x1="{sx(xv):.2f}" y1="{y0}" x2="{sx(xv):.2f}" '
                    f'y2="{y0 + plot_height}" class="grid"/>',
                    f'<text x="{sx(xv):.2f}" y="{y0 + plot_height + 24}" '
                    f'class="tick" text-anchor="middle">{xv:.2f}×</text>',
                    f'<line x1="{x0}" y1="{sy(yv):.2f}" '
                    f'x2="{x0 + plot_width}" y2="{sy(yv):.2f}" class="grid"/>',
                    f'<text x="{x0 - 10}" y="{sy(yv) + 4:.2f}" class="tick" '
                    f'text-anchor="end">{yv:.1f}</text>',
                ]
            )
        for index, row in enumerate(rows):
            key = str(row["slot"] if row["codec"] == "fastvid" else row["preset"])
            x = sx(float(row["ratio"]))
            y = sy(float(row[speed_field]))
            color = colors[key]
            delta = float(row["y_psnr"]) - target
            label = f'{row["label"]} {row["control"]} ({delta:+.2f} dB)'
            label_y = y + [-42, -14, 18, 22, -12][index]
            if row["codec"] == "fastvid":
                shape = f'<circle cx="{x:.2f}" cy="{y:.2f}" r="7" fill="{color}"/>'
            else:
                shape = (
                    f'<rect x="{x - 7:.2f}" y="{y - 7:.2f}" width="14" '
                    f'height="14" fill="{color}"/>'
                )
            output.extend(
                [
                    shape,
                    f'<text x="{x + 10:.2f}" y="{label_y:.2f}" '
                    f'class="label">{html.escape(label)}</text>',
                ]
            )
        output.extend(
            [
                f'<text x="{x0 + plot_width / 2}" y="{y0 + plot_height + 56}" '
                'class="axis" text-anchor="middle">Raw / encoded ratio</text>',
                f'<text x="{x0 - 58}" y="{y0 + plot_height / 2}" class="axis" '
                f'text-anchor="middle" transform="rotate(-90 {x0 - 58} '
                f'{y0 + plot_height / 2})">Luma MP/s</text>',
            ]
        )
        return "".join(output)

    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" '
        f'height="{height}" viewBox="0 0 {width} {height}">'
        "<style>"
        "text{font-family:ui-sans-serif,system-ui,sans-serif;fill:#172033}"
        ".title{font-size:24px;font-weight:700}.subtitle{font-size:13px;fill:#526071}"
        ".panel-title{font-size:17px;font-weight:650}.plot{fill:#fbfdff;stroke:#b9c4d0}"
        ".grid{stroke:#dce3ea;stroke-width:1}.tick{font-size:11px;fill:#667587}"
        ".axis{font-size:12px;font-weight:600}.label{font-size:11px;font-weight:600}"
        "</style>"
        '<rect width="100%" height="100%" fill="#f4f7fa"/>'
        '<text x="42" y="40" class="title">Matched 10-bit external reference</text>'
        '<text x="42" y="64" class="subtitle">1280×720 × 24 frames; YUV 4:2:2 '
        f'all-intra; one thread; OpenAPV selected nearest practical q90 '
        f'Y-PSNR {target:.3f} dB</text>'
        + panel(94, "encode_mpps", "Encode")
        + panel(674, "decode_mpps", "Decode")
        + '<text x="42" y="615" class="subtitle">Circles: preserved Fastvid '
        "frontier binaries. Squares: OpenAPV v0.3.0.0 measured controls. "
        "Labels show Y-PSNR distance from the target.</text>"
        '<text x="42" y="636" class="subtitle">Procedural diagnostic only; '
        "not the 8-bit corpus frontier and not a broad production-content claim."
        "</text></svg>"
    )


def write_summary(
    path: Path,
    matched: dict[int, tuple[list[dict[str, str | float]], float]],
    boundaries: dict[int, tuple[list[dict[str, str | float]], float]],
) -> None:
    fields = [
        "scope",
        "codec",
        "slot",
        "label",
        "preset",
        "control",
        "threads",
        "ratio",
        "bits_per_luma_pixel",
        "encode_mpps",
        "decode_mpps",
        "encoded_stream_mbps",
        "y_psnr",
        "y_psnr_delta",
        "y_block_ssim",
        "max_error",
    ]
    with path.open("w", newline="", encoding="utf-8") as target_file:
        writer = csv.DictWriter(
            target_file, fieldnames=fields, delimiter="\t", lineterminator="\n"
        )
        writer.writeheader()
        for scope, groups in [
            ("matched-q90", matched),
            ("high-fidelity-boundary", boundaries),
        ]:
            for threads in sorted(groups):
                rows, target = groups[threads]
                for row in rows:
                    writer.writerow(
                        {
                            "scope": scope,
                            "codec": row["codec"],
                            "slot": row["slot"],
                            "label": row["label"],
                            "preset": row["preset"],
                            "control": row["control"],
                            "threads": row["threads"],
                            "ratio": f'{float(row["ratio"]):.6f}',
                            "bits_per_luma_pixel": (
                                f'{float(row["bits_per_luma_pixel"]):.6f}'
                            ),
                            "encode_mpps": f'{float(row["encode_mpps"]):.3f}',
                            "decode_mpps": f'{float(row["decode_mpps"]):.3f}',
                            "encoded_stream_mbps": (
                                f'{float(row["encoded_stream_mbps"]):.6f}'
                            ),
                            "y_psnr": (
                                "inf"
                                if math.isinf(float(row["y_psnr"]))
                                else f'{float(row["y_psnr"]):.6f}'
                            ),
                            "y_psnr_delta": (
                                "inf"
                                if math.isinf(float(row["y_psnr"]))
                                else f'{float(row["y_psnr"]) - target:+.6f}'
                            ),
                            "y_block_ssim": f'{float(row["y_block_ssim"]):.8f}',
                            "max_error": f'{float(row["max_error"]):.0f}',
                        }
                    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("results")
    parser.add_argument("svg")
    parser.add_argument("--summary", required=True)
    parser.add_argument("--trials", type=int, default=5)
    args = parser.parse_args()
    results_path = Path(args.results)
    rows = load(results_path, args.trials)
    matched = {threads: select(rows, threads, False) for threads in [1, 4]}
    boundaries = {threads: select(rows, threads, True) for threads in [1, 4]}
    write_summary(Path(args.summary), matched, boundaries)
    graph_rows, target = matched[1]
    Path(args.svg).write_text(render_svg(graph_rows, target), encoding="utf-8")
    print(f"results_sha256={hashlib.sha256(results_path.read_bytes()).hexdigest()}")
    for row in graph_rows:
        print(
            f'{row["label"]} {row["control"]}: ratio={float(row["ratio"]):.3f}x '
            f'encode={float(row["encode_mpps"]):.3f} MP/s '
            f'decode={float(row["decode_mpps"]):.3f} MP/s '
            f'Y={float(row["y_psnr"]):.3f} dB'
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
