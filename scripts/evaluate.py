#!/usr/bin/env python3
"""Canonical correctness, quality, compression, and CUDA performance evaluator.

This file MUST NOT be edited except when specifically requested.

The input manifest is JSON with a ``revision`` string and a ``samples`` array.
Each sample has ``id``, ``path``, ``width``, ``height``, ``format`` (``gray``,
``yuv422``, or ``rgb444``), ``bit_depth``, and ``tiers``. Raw files use
little-endian planar samples; RGB plane order is R, G, B. A sample may set
``batch_frames`` (default 1). Single frames are latency tests and batches are
throughput tests. Relative paths are resolved beside the manifest.

The desired codec module contract is::

    encoded = fastvid.encode(frame, format="yuv422", bit_depth=10, quality=90)
    metadata = fastvid.inspect(encoded)
    decoded = fastvid.decode(encoded)
    streams = fastvid.encode([frame0, ..., frame23], format=..., bit_depth=...)
    frames = fastvid.decode(streams)

A frame is a tuple of planar CUDA ``uint16`` tensors. A single encoded frame is
a one-dimensional byte tensor; a batch is a sequence of independent byte
tensors. Decode obtains dimensions, format, and depth from each bitstream. The
``inspect`` result is a mapping containing ``width``, ``height``, ``format``,
``bit_depth``, ``frame_count``, ``metadata_bytes``, and
``container_overhead_bytes``. The evaluator intentionally fails when any part
of this public interface is absent.

libvship revision and build configuration are deliberately command-line inputs:
the evaluator records them and passes original and roundtrip planes directly to
the in-memory C API without media containers, FFmpeg, or FFMS2.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib
import json
import platform
import statistics
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, Sequence

if __package__:
    from .libvship_direct import DirectVshipMetrics, libvship_version
else:
    from libvship_direct import DirectVshipMetrics, libvship_version

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CORPUS = ROOT / "artifacts" / "corpus-v1"
REQUIRED_MATRIX = {
    ("yuv422", 8), ("yuv422", 10), ("yuv422", 16),
    ("rgb444", 10), ("rgb444", 16),
    ("gray", 8), ("gray", 10), ("gray", 16),
}
SSIMULACRA2_MIN = 90.0
BUTTERAUGLI_MAX = 1.0


@dataclass(frozen=True)
class Sample:
    id: str
    path: Path
    width: int
    height: int
    format: str
    bit_depth: int
    tiers: tuple[str, ...]
    batch_frames: int = 1
    paths: tuple[Path, ...] = ()
    expected_sha256: tuple[str, ...] = ()

    @property
    def plane_shapes(self) -> tuple[tuple[int, int], ...]:
        if self.format == "gray":
            return ((self.height, self.width),)
        if self.format == "yuv422":
            return (
                (self.height, self.width),
                (self.height, (self.width + 1) // 2),
                (self.height, (self.width + 1) // 2),
            )
        if self.format == "rgb444":
            return ((self.height, self.width),) * 3
        raise ValueError(f"{self.id}: unsupported format {self.format!r}")

    @property
    def raw_bytes_per_frame(self) -> int:
        # The canonical files store every required depth in uint16 containers.
        return sum(height * width for height, width in self.plane_shapes) * 2


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def load_manifest(path: Path, tier: str) -> tuple[str, list[Sample]]:
    document = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(document.get("revision"), str):
        raise ValueError("manifest requires a string revision")
    samples: list[Sample] = []
    ids: set[str] = set()
    for row in document.get("samples", []):
        tiers = tuple(row.get("tiers", ("full",)))
        if tier not in tiers:
            continue
        listed_paths = row.get("paths")
        if listed_paths is not None:
            if "path" in row or "batch_frames" in row:
                raise ValueError(f"{row.get('id')}: paths cannot be combined with path or batch_frames")
            if not isinstance(listed_paths, list) or not listed_paths:
                raise ValueError(f"{row.get('id')}: paths must be a non-empty list")
            raw_paths = [Path(value) for value in listed_paths]
            batch_frames = len(raw_paths)
        else:
            raw_paths = [Path(row["path"])]
            batch_frames = int(row.get("batch_frames", 1))
        resolved_paths = tuple(
            (value if value.is_absolute() else path.parent / value).resolve()
            for value in raw_paths
        )
        hashes = row.get("sha256", ())
        if isinstance(hashes, str):
            hashes = (hashes,)
        elif isinstance(hashes, list):
            hashes = tuple(str(value) for value in hashes)
        elif hashes:
            raise ValueError(f"{row.get('id')}: sha256 must be a string or list")
        sample = Sample(
            id=str(row["id"]), path=resolved_paths[0], width=int(row["width"]),
            height=int(row["height"]), format=str(row["format"]).lower(),
            bit_depth=int(row["bit_depth"]), tiers=tiers,
            batch_frames=batch_frames,
            paths=resolved_paths if listed_paths is not None else (),
            expected_sha256=tuple(hashes),
        )
        if sample.id in ids:
            raise ValueError(f"duplicate sample id: {sample.id}")
        if sample.width <= 0 or sample.height <= 0 or sample.batch_frames <= 0:
            raise ValueError(f"{sample.id}: dimensions and batch_frames must be positive")
        if sample.bit_depth not in (8, 10, 16):
            raise ValueError(f"{sample.id}: unsupported bit depth {sample.bit_depth}")
        if sample.expected_sha256 and len(sample.expected_sha256) not in (1, sample.batch_frames):
            raise ValueError(f"{sample.id}: sha256 count differs from input frame count")
        if any(len(value) != 64 for value in sample.expected_sha256):
            raise ValueError(f"{sample.id}: malformed sha256")
        sample.plane_shapes
        ids.add(sample.id)
        samples.append(sample)
    if not samples:
        raise ValueError(f"manifest contains no {tier!r} samples")
    return document["revision"], samples

def samples_exceed_maximum(
    frames: Sequence[tuple[Any, ...]], maximum: int, torch: Any,
) -> bool:
    """Return whether any native-depth plane exceeds the declared range."""
    return any(
        int(plane.to(torch.int32).max().item()) > maximum
        for frame in frames
        for plane in frame
    )

def load_frames(
    sample: Sample, torch: Any,
) -> tuple[list[tuple[Any, ...]], str | list[str]]:
    """Load CUDA codec planes and return the hashes verified while reading."""
    if sample.paths:
        chunks = [path.read_bytes() for path in sample.paths]
        if any(len(chunk) != sample.raw_bytes_per_frame for chunk in chunks):
            raise ValueError(f"{sample.id}: one or more frame files have incorrect size")
        payload = bytearray().join(chunks)
    else:
        payload = bytearray(sample.path.read_bytes())
        chunks = [payload]
    expected = sample.raw_bytes_per_frame * sample.batch_frames
    if len(payload) != expected:
        raise ValueError(f"{sample.id}: expected {expected} raw bytes, found {len(payload)}")
    actual_hashes = tuple(hashlib.sha256(chunk).hexdigest() for chunk in chunks)
    if sample.expected_sha256:
        expected_hashes = sample.expected_sha256
        if len(expected_hashes) == 1 and len(actual_hashes) > 1:
            expected_hashes = expected_hashes * len(actual_hashes)
        if actual_hashes != expected_hashes:
            raise ValueError(f"{sample.id}: extracted input SHA-256 mismatch")

    values = torch.frombuffer(payload, dtype=torch.uint16)
    cuda_frames: list[tuple[Any, ...]] = []
    cursor = 0
    for _ in range(sample.batch_frames):
        cuda_planes = []
        for height, width in sample.plane_shapes:
            count = height * width
            cuda_planes.append(
                values[cursor:cursor + count].clone().view(height, width).cuda()
            )
            cursor += count
        cuda_frames.append(tuple(cuda_planes))
    maximum = (1 << sample.bit_depth) - 1
    if samples_exceed_maximum(cuda_frames, maximum, torch):
        raise ValueError(f"{sample.id}: samples exceed {sample.bit_depth}-bit range")
    source_hashes: str | list[str] = (
        list(actual_hashes) if sample.paths else actual_hashes[0]
    )
    return cuda_frames, source_hashes

def cuda_samples(operation: Callable[[], Any], warmups: int, repetitions: int, torch: Any):
    """Return (seconds, last_result), timed by CUDA events with synchronization."""
    for _ in range(warmups):
        operation()
    torch.cuda.synchronize()
    samples: list[float] = []
    result = None
    for _ in range(repetitions):
        start = torch.cuda.Event(enable_timing=True)
        end = torch.cuda.Event(enable_timing=True)
        torch.cuda.synchronize()
        start.record()
        result = operation()
        end.record()
        end.synchronize()
        samples.append(start.elapsed_time(end) / 1000.0)
    return samples, result


def distribution(seconds: Sequence[float]) -> dict[str, Any]:
    ordered = sorted(seconds)
    percentile = lambda p: ordered[min(len(ordered) - 1, int((len(ordered) - 1) * p))]
    return {
        "samples_seconds": list(seconds),
        "median_seconds": statistics.median(seconds),
        "min_seconds": ordered[0],
        "p95_seconds": percentile(0.95),
        "max_seconds": ordered[-1],
        "gate_statistic": "median",
    }


def quality_group_key(sample: Sample) -> tuple[int, int, int]:
    """Return properties used to summarize direct metric work."""
    return sample.width, sample.height, sample.bit_depth


def assign_quality_scores(
    results: Sequence[dict[str, Any]],
    ssim: Sequence[float],
    butter: Sequence[float],
) -> None:
    """Map direct libvship scores back onto their source samples."""
    expected = sum(int(result["frame_count"]) for result in results)
    if len(ssim) != expected or len(butter) != expected:
        raise RuntimeError(
            f"libvship returned {len(ssim)} SSIMULACRA2 and {len(butter)} "
            f"Butteraugli scores for {expected} input frames"
        )
    cursor = 0
    for result in results:
        count = int(result["frame_count"])
        sample_ssim = ssim[cursor:cursor + count]
        sample_butter = butter[cursor:cursor + count]
        frames = [
            {
                "frame": number, "ssimulacra2": ssim_score,
                "butteraugli": butter_score,
                "passed": (
                    ssim_score > SSIMULACRA2_MIN
                    and butter_score <= BUTTERAUGLI_MAX
                ),
            }
            for number, (ssim_score, butter_score)
            in enumerate(zip(sample_ssim, sample_butter))
        ]
        result["quality"] = {
            "frames": frames,
            "minimum_ssimulacra2": min(sample_ssim),
            "maximum_butteraugli": max(sample_butter),
            "passed": all(frame["passed"] for frame in frames),
        }
        cursor += count


def evaluate_sample(
    sample: Sample, codec: Any, torch: Any, args: argparse.Namespace,
    metrics: DirectVshipMetrics,
) -> dict[str, Any]:
    phases: dict[str, float] = {}
    phase_started = time.perf_counter()
    frames, source_hashes = load_frames(sample, torch)
    phases["load_validate_upload"] = time.perf_counter() - phase_started
    if any(not callable(getattr(codec, name, None)) for name in ("encode", "decode", "inspect")):
        raise NotImplementedError(
            f"{args.codec_module} must expose the desired encode(), decode(), and inspect() API"
        )

    encode_one = lambda frame: codec.encode(
        frame, format=sample.format, bit_depth=sample.bit_depth, quality=args.quality,
        tile_size=tuple(args.tile_size),
    )
    encode_batch = lambda batch: codec.encode(
        batch, format=sample.format, bit_depth=sample.bit_depth, quality=args.quality,
        tile_size=tuple(args.tile_size),
    )
    # Determinism is checked outside all timed regions.
    phase_started = time.perf_counter()
    first = encode_one(frames[0])
    torch.cuda.synchronize()
    second = encode_one(frames[0])
    torch.cuda.synchronize()
    if not torch.equal(first, second):
        raise RuntimeError("two identical encodes produced different bitstreams")
    metadata = codec.inspect(first)
    expected_metadata = {
        "width": sample.width, "height": sample.height, "format": sample.format,
        "bit_depth": sample.bit_depth, "frame_count": 1,
    }
    mismatched = {
        key: (metadata.get(key), value)
        for key, value in expected_metadata.items()
        if metadata.get(key) != value
    }
    if mismatched:
        raise RuntimeError(f"bitstream metadata differs: {mismatched}")
    for key in ("metadata_bytes", "container_overhead_bytes"):
        if key not in metadata:
            raise RuntimeError(f"inspect() omitted required {key}")
    metadata_bytes = int(metadata["metadata_bytes"])
    container_overhead_bytes = int(metadata["container_overhead_bytes"])
    if not 0 <= metadata_bytes + container_overhead_bytes <= int(first.numel()):
        raise RuntimeError("invalid metadata/container byte counts from inspect()")
    decoded = codec.decode(first)
    torch.cuda.synchronize()
    if tuple(tuple(plane.shape) for plane in decoded) != sample.plane_shapes:
        raise RuntimeError("decoded plane dimensions or frame format differ")
    if any(plane.dtype != torch.uint16 or not plane.is_cuda for plane in decoded):
        raise RuntimeError("decoder did not return CUDA uint16 planes")
    decoded_again = codec.decode(first)
    torch.cuda.synchronize()
    if any(not torch.equal(left, right) for left, right in zip(decoded, decoded_again)):
        raise RuntimeError("repeated decode produced different pixels")

    if len(frames) == 1:
        streams = [first]
    else:
        streams = encode_batch(frames)
        if not isinstance(streams, (list, tuple)) or len(streams) != len(frames):
            raise RuntimeError("batch encode must return one independent stream per frame")
    torch.cuda.synchronize()
    encoded_sizes = [int(stream.numel()) for stream in streams]
    raw_bytes = sample.raw_bytes_per_frame * len(frames)
    encoded_bytes = sum(encoded_sizes)
    pixels = sample.width * sample.height * len(frames)
    phases["correctness_and_stream_setup"] = time.perf_counter() - phase_started

    encode_op = (
        (lambda: encode_one(frames[0]))
        if len(frames) == 1 else
        (lambda: encode_batch(frames))
    )
    phase_started = time.perf_counter()
    encode_times, _ = cuda_samples(encode_op, args.warmups, args.repetitions, torch)
    decode_op = (
        (lambda: codec.decode(streams[0]))
        if len(streams) == 1 else
        (lambda: codec.decode(streams))
    )
    decode_times, _ = cuda_samples(decode_op, args.warmups, args.repetitions, torch)
    phases["timing_wall"] = time.perf_counter() - phase_started
    encode_timing, decode_timing = distribution(encode_times), distribution(decode_times)
    mode = "throughput" if len(frames) > 1 else "latency"
    if mode == "throughput":
        encode_timing["gpps"] = pixels / encode_timing["median_seconds"] / 1e9
        decode_timing["gpps"] = pixels / decode_timing["median_seconds"] / 1e9
    elif mode == "latency":
        if len(frames) != 1:
            raise ValueError("latency samples must contain exactly one frame")
        encode_timing["median_ms"] = encode_timing["median_seconds"] * 1000
        decode_timing["median_ms"] = decode_timing["median_seconds"] * 1000

    phase_started = time.perf_counter()
    decoded_frames = [decoded] if len(frames) == 1 else codec.decode(streams)
    torch.cuda.synchronize()
    if not isinstance(decoded_frames, (list, tuple)) or len(decoded_frames) != len(frames):
        raise RuntimeError("batch decode must return one frame per independent stream")
    for number, decoded_frame in enumerate(decoded_frames):
        if tuple(tuple(plane.shape) for plane in decoded_frame) != sample.plane_shapes:
            raise RuntimeError(f"decoded frame {number} dimensions or format differ")
        if any(plane.dtype != torch.uint16 or not plane.is_cuda for plane in decoded_frame):
            raise RuntimeError(f"decoded frame {number} is not CUDA uint16")
    phases["final_decode_validation"] = time.perf_counter() - phase_started
    result = {
        "id": sample.id,
        "path": str(sample.path) if not sample.paths else None,
        "paths": [str(path) for path in sample.paths] if sample.paths else None,
        "source_sha256": source_hashes,
        "format": sample.format,
        "bit_depth": sample.bit_depth,
        "width": sample.width,
        "height": sample.height,
        "frame_count": len(frames),
        "correctness": {"passed": True, "deterministic": True},
        "quality": None,
        "compression": {
            "raw_bytes": raw_bytes, "encoded_bytes": encoded_bytes,
            "encoded_sizes": encoded_sizes,
            "metadata_bytes_first_frame": metadata_bytes,
            "container_overhead_bytes_first_frame": container_overhead_bytes,
            "ratio": raw_bytes / encoded_bytes,
            "bits_per_luma_pixel": encoded_bytes * 8 / pixels,
        },
        "performance": {
            "mode": mode, "warmups": args.warmups,
            "repetitions": args.repetitions,
            "encode": encode_timing, "decode": decode_timing,
        },
        "phase_seconds": phases,
    }
    ssim, butter = metrics.evaluate(frames, decoded_frames, torch)
    assign_quality_scores([result], ssim, butter)
    phases["metric_transfer"] = metrics.last_transfer_seconds
    phases["metric_compute"] = metrics.last_metric_seconds
    return result


def performance_failures(result: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    width, height = result["width"], result["height"]
    perf, fmt = result["performance"], result["format"]
    if perf["mode"] == "throughput" and result["frame_count"] == 24 and (width, height) == (3840, 2160):
        limits = (2.0, 3.0) if fmt == "yuv422" else (1.5, 2.0)
        if perf["encode"]["gpps"] < limits[0]:
            failures.append(f"encode throughput < {limits[0]} GP/s")
        if perf["decode"]["gpps"] < limits[1]:
            failures.append(f"decode throughput < {limits[1]} GP/s")
    if perf["mode"] == "latency" and (width, height) == (1920, 1080) and fmt == "rgb444":
        if perf["encode"]["median_ms"] >= 1.0:
            failures.append("encode latency >= 1.0 ms")
        if perf["decode"]["median_ms"] >= 0.5:
            failures.append("decode latency >= 0.5 ms")
    return failures


def command_output(command: Sequence[str]) -> str | None:
    try:
        return subprocess.run(command, capture_output=True, text=True, timeout=10).stdout.strip()
    except (OSError, subprocess.SubprocessError):
        return None


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    inputs = parser.add_mutually_exclusive_group()
    inputs.add_argument("--manifest", type=Path)
    inputs.add_argument("--corpus", type=Path, default=DEFAULT_CORPUS,
                        help="extracted corpus directory containing manifest.json")
    parser.add_argument("--tier", choices=("rejection", "full"), default="rejection")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--quality", type=int, default=90)
    parser.add_argument("--warmups", type=int, default=5)
    parser.add_argument("--repetitions", type=int, default=20)
    parser.add_argument("--tile-size", type=lambda value: [int(x) for x in value.split("x")], default=[256, 128])
    parser.add_argument("--codec-module", default="fastvid")
    parser.add_argument(
        "--libvship", type=Path,
        default=Path("/usr/local/lib/fastvid-vship-5.0.0/libvship.so"),
    )
    parser.add_argument("--libvship-revision", required=True)
    parser.add_argument("--libvship-build", required=True)
    parser.add_argument("--libvship-gpu-id", type=int, default=0)
    parser.add_argument("--libvship-workers", type=int, default=2)
    args = parser.parse_args(argv)
    if args.repetitions < 20:
        parser.error("--repetitions must be at least 20")
    if args.libvship_gpu_id < 0 or args.libvship_workers < 1:
        parser.error("libvship GPU id must be non-negative and workers positive")
    if args.warmups < 1:
        parser.error("--warmups must be positive")
    if len(args.tile_size) != 2 or min(args.tile_size) <= 0:
        parser.error("--tile-size must be WIDTHxHEIGHT")

    args.manifest = args.manifest or args.corpus / "manifest.json"
    started = time.time()
    failures: list[str] = []
    results: list[dict[str, Any]] = []
    metric_groups: list[dict[str, Any]] = []
    try:
        revision, samples = load_manifest(args.manifest.resolve(), args.tier)
    except Exception as error:
        parser.error(str(error))

    import torch
    try:
        codec = importlib.import_module(args.codec_module)
    except (ImportError, OSError) as error:
        codec = None
        failures.append(f"codec module unavailable: {args.codec_module}: {error}")
    if codec is not None:
        missing_api = [
            name for name in ("encode", "decode", "inspect")
            if not callable(getattr(codec, name, None))
        ]
        if missing_api:
            failures.append(
                f"codec module lacks desired public API: {', '.join(missing_api)}"
            )
    if not torch.cuda.is_available():
        failures.append("CUDA is unavailable")
    libvship_actual = None
    if not args.libvship.is_file():
        failures.append(f"libvship library not found: {args.libvship}")
    else:
        try:
            libvship_actual = libvship_version(args.libvship)
        except (OSError, ValueError) as error:
            failures.append(f"libvship library unavailable: {args.libvship}: {error}")

    if not failures:
        grouped: dict[tuple[int, int, int], list[Sample]] = {}
        for sample in samples:
            grouped.setdefault(quality_group_key(sample), []).append(sample)
        for group_samples in grouped.values():
            group_results: list[dict[str, Any]] = []
            group_metric_seconds = 0.0
            group_frame_count = 0
            source_formats: list[str] = []
            by_format: dict[str, list[Sample]] = {}
            for sample in group_samples:
                by_format.setdefault(sample.format, []).append(sample)
            for format_samples in by_format.values():
                template = format_samples[0]
                source_formats.append(template.format)
                try:
                    with DirectVshipMetrics(
                        args.libvship, template.width, template.height,
                        template.format, template.bit_depth,
                        args.libvship_gpu_id, args.libvship_workers,
                    ) as metrics:
                        for sample in format_samples:
                            try:
                                result = evaluate_sample(
                                    sample, codec, torch, args, metrics,
                                )
                                group_results.append(result)
                            except Exception as error:
                                failures.append(
                                    f"{sample.id}: {type(error).__name__}: {error}"
                                )
                        group_metric_seconds += metrics.metric_seconds
                        group_frame_count += metrics.frame_count
                except Exception as error:
                    for sample in format_samples:
                        if not any(result["id"] == sample.id for result in group_results):
                            failures.append(
                                f"{sample.id}: direct libvship: "
                                f"{type(error).__name__}: {error}"
                            )
            if group_results:
                metric_groups.append({
                    "width": group_samples[0].width,
                    "height": group_samples[0].height,
                    "bit_depth": group_samples[0].bit_depth,
                    "source_formats": source_formats,
                    "sample_count": len(group_results),
                    "frame_count": group_frame_count,
                    "metric_seconds": group_metric_seconds,
                    "metric_frames_per_second": (
                        group_frame_count / group_metric_seconds
                        if group_metric_seconds else None
                    ),
                })
                results.extend(group_results)
                for result in group_results:
                    if not result["quality"]["passed"]:
                        failures.append(f"{result['id']}: perceptual quality gate failed")
                    failures.extend(
                        f"{result['id']}: {item}"
                        for item in performance_failures(result)
                    )

    sample_order = {sample.id: number for number, sample in enumerate(samples)}
    results.sort(key=lambda result: sample_order[result["id"]])

    covered = {(row["format"], row["bit_depth"]) for row in results}
    if not any(row["frame_count"] > 1 for row in results):
        failures.append("coverage: no many-frame throughput sample")
    if not any(row["frame_count"] == 1 for row in results):
        failures.append("coverage: no single-frame latency sample")
    if args.tier == "full":
        for fmt, depth in sorted(REQUIRED_MATRIX - covered):
            failures.append(f"missing required full-tier coverage: {fmt} {depth}-bit")
        required_performance = {
            ("yuv422", 3840, 2160, 24),
            ("rgb444", 3840, 2160, 24),
            ("rgb444", 1920, 1080, 1),
        }
        actual_performance = {
            (row["format"], row["width"], row["height"], row["frame_count"])
            for row in results
        }
        for fmt, width, height, frames in sorted(required_performance - actual_performance):
            failures.append(
                f"missing required performance case: {fmt} {width}x{height} x{frames}"
            )
    total_raw = sum(row["compression"]["raw_bytes"] for row in results)
    total_encoded = sum(row["compression"]["encoded_bytes"] for row in results)
    phase_names = {name for row in results for name in row["phase_seconds"]}
    phase_summary = {
        name: sum(row["phase_seconds"].get(name, 0.0) for row in results)
        for name in sorted(phase_names)
    }
    report = {
        "schema_version": 3,
        "passed": not failures,
        "tier": args.tier,
        "configuration": {
            "quality": args.quality, "warmups": args.warmups,
            "repetitions": args.repetitions, "tile_size": args.tile_size,
            "codec_module": args.codec_module,
            "metric_interface": "direct in-memory libvship C API",
            "metric_colorspace": (
                "native full-range BT.709 YUV422 or RGB; gray planes are repeated as RGB"
            ),
            "libvship_path": str(args.libvship.resolve()),
            "libvship_revision": args.libvship_revision,
            "libvship_build": args.libvship_build,
            "libvship_actual_version": libvship_actual,
            "libvship_gpu_id": args.libvship_gpu_id,
            "libvship_workers_per_metric": args.libvship_workers,
            "libvship_parallel_metrics": 2,
            "butteraugli_score": "maximum distance among q=2, q=3, and infinity norms",
        },
        "corpus": {
            "revision": revision,
            "manifest": str(args.manifest.resolve()),
            "manifest_sha256": sha256_file(args.manifest.resolve()),
            "selected_ids": [sample.id for sample in samples],
        },
        "revisions": {
            "git": command_output(["git", "-C", str(ROOT), "rev-parse", "HEAD"]),
            "python": sys.version,
            "torch": getattr(torch, "__version__", None),
            "cuda": getattr(torch.version, "cuda", None),
        },
        "hardware": {
            "platform": platform.platform(),
            "gpu": torch.cuda.get_device_name(0) if torch.cuda.is_available() else None,
            "gpu_properties": (
                str(torch.cuda.get_device_properties(0)) if torch.cuda.is_available() else None
            ),
            "nvidia_smi": command_output([
                "nvidia-smi", "--query-gpu=name,clocks.current.graphics,power.limit",
                "--format=csv,noheader",
            ]),
        },
        "quality_summary": {
            "minimum_ssimulacra2": min(
                (row["quality"]["minimum_ssimulacra2"] for row in results), default=None
            ),
            "maximum_butteraugli": max(
                (row["quality"]["maximum_butteraugli"] for row in results), default=None
            ),
        },
        "metric_groups": metric_groups,
        "phase_summary_seconds": phase_summary,
        "compression_summary": {
            "raw_bytes": total_raw, "encoded_bytes": total_encoded,
            "ratio": total_raw / total_encoded if total_encoded else None,
            "bits_per_luma_pixel": (
                total_encoded * 8 / sum(
                    row["width"] * row["height"] * row["frame_count"] for row in results
                ) if results else None
            ),
        },
        "samples": results,
        "failures": failures,
        "elapsed_seconds": time.time() - started,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({
        "passed": report["passed"], "samples": len(results),
        "minimum_ssimulacra2": report["quality_summary"]["minimum_ssimulacra2"],
        "maximum_butteraugli": report["quality_summary"]["maximum_butteraugli"],
        "compression_ratio": report["compression_summary"]["ratio"],
        "output": str(args.output),
        "failures": failures,
    }, indent=2))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
