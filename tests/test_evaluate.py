"""Regression tests for the canonical evaluator's metric interchange."""

from pathlib import Path
import pytest

from scripts.evaluate import (
    EDIT_CYCLES, Sample, apply_edit, assign_quality_scores, cycle_quality,
    derived_edit_seed, load_manifest, make_edit_trace, normalize_edited_frame,
    plane_shapes,
    quality_group_key, samples_exceed_maximum,
)
from scripts.libvship_direct import _colorspace, libvship_version



@pytest.mark.parametrize(
    ("format_name", "family", "matrix", "subw", "subh"),
    (
        ("yuv422", 0, 1, 1, 0),
        ("rgb444", 1, 0, 0, 0),
        ("gray", 1, 0, 0, 0),
    ),
)
def test_direct_colorspace_preserves_native_layout(
    format_name, family, matrix, subw, subh,
):
    colorspace = _colorspace(1920, 1080, format_name, 10)
    assert colorspace.width == 1920
    assert colorspace.height == 1080
    assert colorspace.sample == 5
    assert colorspace.range == 1
    assert colorspace.color_family == family
    assert colorspace.yuv_matrix == matrix
    assert colorspace.subsampling.subw == subw
    assert colorspace.subsampling.subh == subh


@pytest.mark.parametrize(("depth", "sample_type"), ((8, 2), (10, 5), (16, 11)))
def test_direct_colorspace_preserves_bit_depth(depth, sample_type):
    assert _colorspace(16, 16, "rgb444", depth).sample == sample_type



def test_installed_libvship_version_matches_direct_baseline():
    library = Path("/usr/local/lib/fastvid-vship-5.0.0/libvship.so")
    if not library.is_file():
        pytest.skip("validated libvship unavailable")
    assert libvship_version(library) == {
        "major": 5, "minor": 0, "patch": 0, "backend": "CUDA",
    }

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


def test_reporting_group_key_ignores_native_source_format():
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


def test_edit_trace_is_stable_varied_and_never_upscales():
    sample = Sample(
        id="batch", path=Path("unused.raw"), width=1920, height=1080,
        format="rgb444", bit_depth=10, tiers=("rejection",), batch_frames=24,
    )
    trace = make_edit_trace(sample, 1234)
    assert trace == make_edit_trace(sample, 1234)
    assert len(trace) == EDIT_CYCLES
    assert {kind: sum(row["type"] == kind for row in trace) for kind in {
        "mild_crop", "mild_resize", "rotate", "recolor", "patch_recolor",
        "patch_blur", "horizontal_flip", "sharpen",
    }} == {
        "mild_crop": 1, "mild_resize": 1, "rotate": 1, "recolor": 2,
        "patch_recolor": 2, "patch_blur": 1, "horizontal_flip": 1,
        "sharpen": 1,
    }
    for edit in trace:
        before, after = edit["input_geometry"], edit["output_geometry"]
        if edit["type"] in ("mild_crop", "mild_resize"):
            assert 0.95 <= after["width"] / before["width"] <= 1.0
            assert 0.95 <= after["height"] / before["height"] <= 1.0
    assert derived_edit_seed(1234, "batch", 1) != derived_edit_seed(1234, "other", 1)


def test_yuv_trace_preserves_horizontal_subsampling_rules():
    sample = Sample(
        id="yuv", path=Path("unused.raw"), width=320, height=180,
        format="yuv422", bit_depth=10, tiers=("rejection",),
    )
    trace = make_edit_trace(sample, 9)
    assert all(
        edit.get("degrees") == 180
        for edit in trace if edit["type"] == "rotate"
    )
    assert all(
        edit["output_geometry"]["width"] % 2 == 0
        for edit in trace
    )
    assert all(
        edit.get("left", 0) % 2 == 0
        for edit in trace if edit["type"] == "crop"
    )


def test_edit_replay_preserves_whole_batch_geometry_on_cuda():
    torch = pytest.importorskip("torch")
    if not torch.cuda.is_available():
        pytest.skip("CUDA unavailable")
    sample = Sample(
        id="small-batch", path=Path("unused.raw"), width=128, height=96,
        format="rgb444", bit_depth=10, tiers=("rejection",), batch_frames=2,
    )
    frames = [tuple(
        torch.full(
            (sample.height, sample.width), value,
            dtype=torch.uint16, device="cuda",
        )
        for value in (100, 200, 300)
    ) for _ in range(2)]
    for edit in make_edit_trace(sample, 55):
        frames = [
            apply_edit(frame, sample.format, sample.bit_depth, edit, torch)
            for frame in frames
        ]
        output = edit["output_geometry"]
        expected = plane_shapes(sample.format, output["width"], output["height"])
        assert all(tuple(tuple(plane.shape) for plane in frame) == expected for frame in frames)
        frames = [normalize_edited_frame(frame, sample.bit_depth, torch) for frame in frames]
        assert all(all(plane.is_contiguous() for plane in frame) for frame in frames)
        assert all(
            all(int(plane.to(torch.int32).max().item()) <= 1023 for plane in frame)
            for frame in frames
        )


def test_cycle_quality_gates_on_worst_frame():
    quality = cycle_quality(3, [95.0, 89.0], [0.2, 0.4])
    assert quality["minimum_ssimulacra2"] == 89.0
    assert not quality["passed"]


def test_direct_libvship_identical_and_perturbed_controls():
    torch = pytest.importorskip("torch")
    if not torch.cuda.is_available():
        pytest.skip("CUDA unavailable")
    library = Path("/usr/local/lib/fastvid-vship-5.0.0/libvship.so")
    if not library.is_file():
        pytest.skip("validated libvship unavailable")
    from scripts.libvship_direct import DirectVshipMetrics

    source = tuple(
        torch.full((256, 256), value, dtype=torch.uint16, device="cuda")
        for value in (400, 500, 600)
    )
    perturbed = (
        torch.full((256, 256), 408, dtype=torch.uint16, device="cuda"),
        source[1].clone(), source[2].clone(),
    )
    with DirectVshipMetrics(
        library, 256, 256, "rgb444", 10, gpu_id=0, workers=1,
    ) as metrics:
        identical_ssim, identical_butter = metrics.evaluate(
            [source], [source], torch,
        )
        changed_ssim, changed_butter = metrics.evaluate(
            [source], [perturbed], torch,
        )
    assert identical_ssim[0] > 90.0
    assert identical_butter == [0.0]
    assert changed_ssim[0] < identical_ssim[0]
    assert changed_butter[0] > 0.0
