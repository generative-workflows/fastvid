#!/usr/bin/env python3
"""Canonical correctness, quality, compression, and CUDA performance evaluator.

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

FFVShip revision and build configuration are deliberately command-line inputs:
the evaluator records them but never downloads or silently substitutes a
metric implementation.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib
import json
import platform
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, Sequence

ROOT = Path(__file__).resolve().parents[1]
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
        sample_path = Path(row["path"])
        if not sample_path.is_absolute():
            sample_path = path.parent / sample_path
        sample = Sample(
            id=str(row["id"]),
            path=sample_path.resolve(),
            width=int(row["width"]),
            height=int(row["height"]),
            format=str(row["format"]).lower(),
            bit_depth=int(row["bit_depth"]),
            tiers=tiers,
            batch_frames=int(row.get("batch_frames", 1)),
        )
        if sample.id in ids:
            raise ValueError(f"duplicate sample id: {sample.id}")
        if sample.width <= 0 or sample.height <= 0 or sample.batch_frames <= 0:
            raise ValueError(f"{sample.id}: dimensions and batch_frames must be positive")
        if sample.bit_depth not in (8, 10, 16):
            raise ValueError(f"{sample.id}: unsupported bit depth {sample.bit_depth}")
        sample.plane_shapes  # validate format now
        ids.add(sample.id)
        samples.append(sample)
    if not samples:
        raise ValueError(f"manifest contains no {tier!r} samples")
    return document["revision"], samples


def load_frames(sample: Sample, torch: Any) -> list[tuple[Any, ...]]:
    payload = bytearray(sample.path.read_bytes())
    expected = sample.raw_bytes_per_frame * sample.batch_frames
    if len(payload) != expected:
        raise ValueError(f"{sample.id}: expected {expected} raw bytes, found {len(payload)}")
    values = torch.frombuffer(payload, dtype=torch.uint16)
    frames: list[tuple[Any, ...]] = []
    cursor = 0
    for _ in range(sample.batch_frames):
        planes = []
        for height, width in sample.plane_shapes:
            count = height * width
            planes.append(values[cursor:cursor + count].clone().view(height, width).cuda())
            cursor += count
        frames.append(tuple(planes))
    maximum = (1 << sample.bit_depth) - 1
    if any(int(plane.max().item()) > maximum for frame in frames for plane in frame):
        raise ValueError(f"{sample.id}: samples exceed {sample.bit_depth}-bit range")
    return frames


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


def run_ffmpeg(raw: Path, output: Path, sample: Sample, ffmpeg: str) -> None:
    pixel_formats = {
        ("gray", 8): "gray", ("gray", 10): "gray10le", ("gray", 16): "gray16le",
        ("yuv422", 8): "yuv422p", ("yuv422", 10): "yuv422p10le",
        ("yuv422", 16): "yuv422p16le",
        ("rgb444", 10): "gbrp10le", ("rgb444", 16): "gbrp16le",
    }
    source_format = pixel_formats[(sample.format, sample.bit_depth)]
    command = [
        ffmpeg, "-v", "error", "-y", "-f", "rawvideo",
        "-pixel_format", source_format, "-video_size", f"{sample.width}x{sample.height}",
        "-framerate", "1", "-color_range", "pc", "-colorspace", "bt709",
        "-color_primaries", "bt709", "-color_trc", "bt709",
        "-i", str(raw), "-frames:v", str(sample.batch_frames),
        "-vf", "format=gbrp16le", "-color_range", "pc",
        "-colorspace", "bt709", "-color_primaries", "bt709", "-color_trc", "bt709",
        "-c:v", "ffv1", "-level", "3", "-pix_fmt", "gbrp16le", str(output),
    ]
    subprocess.run(command, check=True, capture_output=True, text=True)


def parse_ffvship_scores(path: Path, metric: str) -> list[float]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if isinstance(data, dict):
        for key in (metric, metric.lower(), "frames", "scores"):
            if key in data:
                data = data[key]
                break
    if not isinstance(data, list):
        raise ValueError(f"unexpected FFVShip {metric} JSON")
    scores = []
    for row in data:
        if isinstance(row, (int, float)):
            scores.append(float(row))
        elif metric == "Butteraugli":
            # Releases have changed the order/names of the three norms. Using
            # the largest emitted distance is deterministic and conservative.
            scores.append(max(float(value) for value in row))
        else:
            scores.append(float(row[0]))
    return scores


def ffvship_metric(
    reference: Path, distorted: Path, metric: str, binary: str, directory: Path,
) -> list[float]:
    output = directory / f"{metric.lower()}.json"
    command = [
        binary, "--source", str(reference), "--encoded", str(distorted),
        "-m", metric, "--json", str(output),
    ]
    completed = subprocess.run(command, capture_output=True, text=True)
    if completed.returncode:
        raise RuntimeError(f"FFVShip {metric} failed: {completed.stderr.strip()}")
    return parse_ffvship_scores(output, metric)


def metric_raw(frame: Sequence[Any], sample: Sample) -> bytes:
    """Serialize a frame in the exact raw layout passed to FFmpeg."""
    planes = frame
    if sample.format == "rgb444":
        # The public API is conventional R,G,B; FFmpeg's planar format is G,B,R.
        planes = (frame[1], frame[2], frame[0])
    arrays = [plane.detach().cpu().contiguous().numpy() for plane in planes]
    if sample.bit_depth == 8:
        return b"".join(array.astype("uint8", copy=False).tobytes() for array in arrays)
    return b"".join(array.tobytes() for array in arrays)


def evaluate_sample(
    sample: Sample, codec: Any, torch: Any, args: argparse.Namespace,
) -> dict[str, Any]:
    frames = load_frames(sample, torch)
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

    encode_op = (
        (lambda: encode_one(frames[0]))
        if len(frames) == 1 else
        (lambda: encode_batch(frames))
    )
    encode_times, _ = cuda_samples(encode_op, args.warmups, args.repetitions, torch)
    decode_op = (
        (lambda: codec.decode(streams[0]))
        if len(streams) == 1 else
        (lambda: codec.decode(streams))
    )
    decode_times, _ = cuda_samples(decode_op, args.warmups, args.repetitions, torch)
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

    with tempfile.TemporaryDirectory(prefix="fastvid-quality-") as temp_name:
        temp = Path(temp_name)
        decoded_frames = [decoded] if len(frames) == 1 else codec.decode(streams)
        torch.cuda.synchronize()
        if not isinstance(decoded_frames, (list, tuple)) or len(decoded_frames) != len(frames):
            raise RuntimeError("batch decode must return one frame per independent stream")
        for number, decoded_frame in enumerate(decoded_frames):
            if tuple(tuple(plane.shape) for plane in decoded_frame) != sample.plane_shapes:
                raise RuntimeError(f"decoded frame {number} dimensions or format differ")
            if any(plane.dtype != torch.uint16 or not plane.is_cuda for plane in decoded_frame):
                raise RuntimeError(f"decoded frame {number} is not CUDA uint16")
        reference_raw, decoded_raw = temp / "reference.raw", temp / "decoded.raw"
        reference_raw.write_bytes(b"".join(metric_raw(frame, sample) for frame in frames))
        decoded_raw.write_bytes(b"".join(metric_raw(frame, sample) for frame in decoded_frames))
        reference_video, decoded_video = temp / "reference.mkv", temp / "decoded.mkv"
        run_ffmpeg(reference_raw, reference_video, sample, args.ffmpeg)
        run_ffmpeg(decoded_raw, decoded_video, sample, args.ffmpeg)
        ssim = ffvship_metric(reference_video, decoded_video, "SSIMULACRA2", args.ffvship, temp)
        butter = ffvship_metric(reference_video, decoded_video, "Butteraugli", args.ffvship, temp)
    if len(ssim) != len(frames) or len(butter) != len(frames):
        raise RuntimeError("FFVShip did not return exactly one score per input frame")

    per_frame_quality = [
        {
            "frame": number, "ssimulacra2": ssim_score,
            "butteraugli": butter_score,
            "passed": ssim_score > SSIMULACRA2_MIN and butter_score <= BUTTERAUGLI_MAX,
        }
        for number, (ssim_score, butter_score) in enumerate(zip(ssim, butter))
    ]
    quality_pass = all(row["passed"] for row in per_frame_quality)
    return {
        "id": sample.id,
        "path": str(sample.path),
        "source_sha256": sha256_file(sample.path),
        "format": sample.format,
        "bit_depth": sample.bit_depth,
        "width": sample.width,
        "height": sample.height,
        "frame_count": len(frames),
        "correctness": {"passed": True, "deterministic": True},
        "quality": {
            "frames": per_frame_quality,
            "minimum_ssimulacra2": min(ssim),
            "maximum_butteraugli": max(butter),
            "passed": quality_pass,
        },
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
    }


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
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--tier", choices=("rejection", "full"), default="rejection")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--quality", type=int, default=90)
    parser.add_argument("--warmups", type=int, default=5)
    parser.add_argument("--repetitions", type=int, default=20)
    parser.add_argument("--tile-size", type=lambda value: [int(x) for x in value.split("x")], default=[256, 128])
    parser.add_argument("--codec-module", default="fastvid")
    parser.add_argument("--ffvship", default="FFVship")
    parser.add_argument("--ffvship-revision", required=True)
    parser.add_argument("--ffvship-build", required=True)
    parser.add_argument("--ffmpeg", default="ffmpeg")
    args = parser.parse_args(argv)
    if args.repetitions < 20:
        parser.error("--repetitions must be at least 20")
    if args.warmups < 1:
        parser.error("--warmups must be positive")
    if len(args.tile_size) != 2 or min(args.tile_size) <= 0:
        parser.error("--tile-size must be WIDTHxHEIGHT")

    started = time.time()
    failures: list[str] = []
    results: list[dict[str, Any]] = []
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
    if shutil.which(args.ffvship) is None:
        failures.append(f"FFVShip binary not found: {args.ffvship}")
    if shutil.which(args.ffmpeg) is None:
        failures.append(f"FFmpeg binary not found: {args.ffmpeg}")

    if not failures:
        for sample in samples:
            try:
                result = evaluate_sample(sample, codec, torch, args)
                results.append(result)
                if not result["quality"]["passed"]:
                    failures.append(f"{sample.id}: perceptual quality gate failed")
                failures.extend(f"{sample.id}: {item}" for item in performance_failures(result))
            except Exception as error:
                failures.append(f"{sample.id}: {type(error).__name__}: {error}")

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
    report = {
        "schema_version": 1,
        "passed": not failures,
        "tier": args.tier,
        "configuration": {
            "quality": args.quality, "warmups": args.warmups,
            "repetitions": args.repetitions, "tile_size": args.tile_size,
            "codec_module": args.codec_module,
            "metric_conversion": (
                "raw planar little-endian uint16 at original format/depth -> "
                "FFmpeg format=gbrp16le, full-range BT.709 -> lossless FFV1 level 3"
            ),
            "ffvship_revision": args.ffvship_revision,
            "ffvship_build": args.ffvship_build,
            "butteraugli_score": "maximum distance among all norms emitted by FFVShip",
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
            "ffmpeg": command_output([args.ffmpeg, "-version"]),
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
