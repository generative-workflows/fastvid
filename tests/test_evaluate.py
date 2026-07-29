"""Regression tests for the canonical evaluator's metric interchange."""

from pathlib import Path
import subprocess

import pytest

from scripts.evaluate import (
    Sample,
    metric_pixel_format,
    metric_raw,
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
