"""Regression tests for the canonical evaluator's metric interchange."""

from pathlib import Path
import pytest

from scripts.evaluate import (
    Sample, assign_quality_scores, load_manifest, quality_group_key,
    samples_exceed_maximum,
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
