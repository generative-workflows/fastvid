"""Regression tests for the canonical evaluator's metric interchange."""

from pathlib import Path
import subprocess

import pytest

from scripts.evaluate import (
    Sample,
    assign_quality_scores,
    concatenate_metric_videos,
    load_manifest,
    metric_pixel_format,
    metric_raw,
    quality_group_key,
    run_ffmpeg,
    samples_exceed_maximum,
)


@pytest.mark.parametrize(
    ("bit_depth", "expected"),
    ((8, "yuv444p"), (10, "yuv444p10le"), (16, "yuv444p16le")),
)
def test_metric_pixel_format_preserves_source_depth(bit_depth, expected):
    assert metric_pixel_format(bit_depth) == expected


def test_gray16_metric_conversion_preserves_low_bits(tmp_path: Path):
    """A one-LSB 16-bit difference must survive the metric conversion."""
    sample = Sample(
        id="gray16-lsb",
        path=tmp_path / "unused.raw",
        width=16,
        height=16,
        format="gray",
        bit_depth=16,
        tiers=("rejection",),
    )
    left = tmp_path / "left.raw"
    right = tmp_path / "right.raw"
    left_values = [30_000] * (16 * 16)
    right_values = left_values.copy()
    right_values[100] += 1
    left.write_bytes(b"".join(value.to_bytes(2, "little") for value in left_values))
    right.write_bytes(b"".join(value.to_bytes(2, "little") for value in right_values))
    left_video = tmp_path / "left.mkv"
    right_video = tmp_path / "right.mkv"
    run_ffmpeg(left, left_video, sample, "ffmpeg")
    run_ffmpeg(right, right_video, sample, "ffmpeg")

    def decoded_bytes(video: Path) -> bytes:
        return subprocess.run(
            [
                "ffmpeg", "-v", "error", "-i", str(video), "-frames:v", "1",
                "-f", "rawvideo", "-pix_fmt", "yuv444p16le", "-",
            ],
            check=True,
            capture_output=True,
        ).stdout

    assert decoded_bytes(left_video) != decoded_bytes(right_video)


def test_rgb_metric_raw_reorders_api_rgb_to_ffmpeg_gbr():
    torch = pytest.importorskip("torch")
    sample = Sample(
        id="rgb-order",
        path=Path("unused.raw"),
        width=1,
        height=1,
        format="rgb444",
        bit_depth=10,
        tiers=("rejection",),
    )
    red = torch.tensor([[1]], dtype=torch.uint16)
    green = torch.tensor([[2]], dtype=torch.uint16)
    blue = torch.tensor([[3]], dtype=torch.uint16)
    assert metric_raw((red, green, blue), sample) == (
        b"\x02\x00\x03\x00\x01\x00"
    )


def test_sample_range_validation_is_linear_in_frame_planes():
    class FakePlane:
        calls = 0

        def __init__(self, value):
            self.value = value

        def to(self, _dtype):
            FakePlane.calls += 1
            return self

        def max(self):
            return self

        def item(self):
            return self.value

    class FakeTorch:
        int32 = object()

    frames = [tuple(FakePlane(value) for value in pair) for pair in ((1, 2), (3, 4), (5, 6))]
    assert not samples_exceed_maximum(frames, 6, FakeTorch)
    assert FakePlane.calls == 6


def test_manifest_accepts_multifile_batch_and_hashes(tmp_path: Path):
    first = tmp_path / "first.raw"
    second = tmp_path / "second.raw"
    first.write_bytes(b"\x00\x00")
    second.write_bytes(b"\x01\x00")
    import hashlib
    document = {
        "revision": "test",
        "samples": [{
            "id": "batch", "paths": [first.name, second.name],
            "sha256": [hashlib.sha256(first.read_bytes()).hexdigest(),
                       hashlib.sha256(second.read_bytes()).hexdigest()],
            "width": 1, "height": 1, "format": "gray", "bit_depth": 8,
            "tiers": ["rejection"],
        }],
    }
    import json
    manifest = tmp_path / "manifest.json"
    manifest.write_text(json.dumps(document))
    revision, samples = load_manifest(manifest, "rejection")
    assert revision == "test"
    assert samples[0].batch_frames == 2
    assert samples[0].paths == (first, second)
    assert len(samples[0].expected_sha256) == 2


def test_quality_group_key_ignores_normalized_source_format():
    sample = Sample(
        id="group", path=Path("unused.raw"), width=1920, height=1080,
        format="rgb444", bit_depth=10, tiers=("rejection",),
    )
    gray = Sample(
        id="gray", path=Path("unused.raw"), width=1920, height=1080,
        format="gray", bit_depth=10, tiers=("rejection",),
    )
    assert quality_group_key(sample) == (1920, 1080, 10)
    assert quality_group_key(gray) == quality_group_key(sample)


def test_consolidated_scores_map_back_to_sample_frames():
    results = [
        {"id": "one", "frame_count": 1},
        {"id": "two", "frame_count": 2},
    ]
    assign_quality_scores(
        results, [95.0, 91.0, 89.0], [0.2, 0.9, 0.1],
    )
    assert results[0]["quality"]["frames"][0]["ssimulacra2"] == 95.0
    assert [row["frame"] for row in results[1]["quality"]["frames"]] == [0, 1]
    assert results[0]["quality"]["passed"]
    assert not results[1]["quality"]["passed"]
    assert results[1]["quality"]["minimum_ssimulacra2"] == 89.0


def test_metric_segment_concatenation_preserves_frame_count(tmp_path: Path):
    sample = Sample(
        id="concat", path=tmp_path / "unused.raw", width=16, height=16,
        format="gray", bit_depth=10, tiers=("rejection",),
    )
    segments = []
    for number, value in enumerate((100, 200)):
        raw = tmp_path / f"segment-{number}.raw"
        raw.write_bytes(value.to_bytes(2, "little") * (16 * 16))
        video = tmp_path / f"segment-{number}.mkv"
        run_ffmpeg(raw, video, sample, "ffmpeg")
        segments.append(video)
    output = tmp_path / "joined.mkv"
    concatenate_metric_videos(segments, output, "ffmpeg", tmp_path)
    probe = subprocess.run(
        [
            "ffprobe", "-v", "error", "-count_frames",
            "-select_streams", "v:0", "-show_entries",
            "stream=nb_read_frames", "-of", "default=nw=1:nk=1", str(output),
        ],
        check=True, capture_output=True, text=True,
    )
    assert probe.stdout.strip() == "2"
