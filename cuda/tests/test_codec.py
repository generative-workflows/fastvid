import struct

import pytest
import torch

from fastvid_cuda import decode, encode


def _source_planes(layout, width, height, bit_depth, pattern="polynomial"):
    maximum = (1 << bit_depth) - 1
    plane_widths = [width] if layout == "gray" else [
        width,
        (width + 1) // 2 if layout == "yuv422" else width,
        (width + 1) // 2 if layout == "yuv422" else width,
    ]

    def make_plane(count, seed):
        index = torch.arange(count, dtype=torch.int64)
        if pattern == "zero":
            values = torch.zeros_like(index)
        elif pattern == "ramp":
            values = index * (seed * 2 + 1) + seed * 17
        else:
            values = index * (977 + seed * 13) + index.square() * 17 + seed * 131
        return (values % (maximum + 1)).to(torch.uint16)

    return tuple(
        make_plane(plane_width * height, seed).view(height, plane_width)
        for seed, plane_width in enumerate(plane_widths, 1)
    )


def _shard_modes(stream):
    tile_count = struct.unpack_from("<I", stream, 28)[0]
    modes = set()
    for tile in range(tile_count):
        entry = 32 + tile * 32
        width, height = struct.unpack_from("<II", stream, entry + 12)
        cursor = struct.unpack_from("<Q", stream, entry + 20)[0]
        tile_end = cursor + struct.unpack_from("<I", stream, entry + 28)[0]
        decoded = 0
        while decoded < width * height:
            mode = stream[cursor]
            body_length = struct.unpack_from("<H", stream, cursor + 1)[0]
            modes.add(mode)
            cursor += 3 + body_length
            decoded += min(4096, width * height - decoded)
        assert cursor == tile_end
    return modes


@pytest.mark.parametrize("layout", ("gray", "yuv422", "rgb444"))
@pytest.mark.parametrize("bit_depth", (8, 10, 12, 16))
def test_q100_roundtrip_is_exact_and_emits_version_one(layout, bit_depth):
    source = _source_planes(layout, 67, 35, bit_depth)
    cuda_source = tuple(plane.cuda() for plane in source)
    first = encode(cuda_source, layout=layout, bit_depth=bit_depth, quality=100)
    second = encode(cuda_source, layout=layout, bit_depth=bit_depth, quality=100)
    assert first.is_cuda and first.dtype == torch.uint8
    assert bytes(first.cpu().numpy()) == bytes(second.cpu().numpy())
    assert int(first[4].item()) == 1
    for placement in (first.cpu(), first):
        for predictor in ("wavefront", "serial"):
            actual = decode(placement, predictor=predictor)
            assert len(actual) == len(source)
            for result, expected in zip(actual, source):
                assert result.is_cuda and result.dtype == torch.uint16
                assert torch.equal(result.cpu(), expected)


def test_lossy_stream_is_deterministic_and_exercises_entropy_modes():
    selected_modes = set()
    for quality, pattern in ((90, "ramp"), (100, "zero"), (100, "polynomial")):
        source = _source_planes("yuv422", 263, 133, 10, pattern)
        stream = encode(
            tuple(plane.cuda() for plane in source),
            layout="yuv422", bit_depth=10, quality=quality,
        )
        payload = bytes(stream.cpu().numpy())
        selected_modes.update(_shard_modes(payload))
        wavefront = decode(stream, predictor="wavefront")
        serial = decode(stream, predictor="serial")
        assert all(torch.equal(left, right) for left, right in zip(wavefront, serial))
    assert 0 in selected_modes
    assert 18 in selected_modes
    assert any(1 <= mode <= 17 for mode in selected_modes)


def test_decode_rejects_other_versions_and_malformed_streams():
    source = _source_planes("yuv422", 263, 133, 10)
    stream = bytearray(encode(
        tuple(plane.cuda() for plane in source),
        layout="yuv422", bit_depth=10, quality=100,
    ).cpu().numpy())
    first_payload = struct.unpack_from("<Q", stream, 32 + 20)[0]
    mutations = []

    old_version = bytearray(stream)
    old_version[4] = 7
    mutations.append(old_version)
    bad_directory = bytearray(stream)
    bad_directory[32 + 3] = 1
    mutations.append(bad_directory)
    bad_offset = bytearray(stream)
    struct.pack_into("<Q", bad_offset, 32 + 20, first_payload + 1)
    mutations.append(bad_offset)
    bad_mode = bytearray(stream)
    bad_mode[first_payload] = 255
    mutations.append(bad_mode)
    mutations.append(bytearray(stream[:-1]))
    mutations.append(bytearray(stream + b"\x00"))

    for malformed in mutations:
        with pytest.raises(RuntimeError):
            decode(torch.frombuffer(malformed, dtype=torch.uint8).cuda())
