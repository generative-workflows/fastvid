import struct
import subprocess
import tempfile
from pathlib import Path

import torch

from fastvid_cuda import decode_v5, encode_v5


ROOT = Path(__file__).resolve().parents[2]
BINARY = ROOT / "target" / "release" / "fastvid"


def _source_planes(width, height, bit_depth, pattern):
    maximum = (1 << bit_depth) - 1
    chroma_width = (width + 1) // 2

    def make_plane(count, seed):
        index = torch.arange(count, dtype=torch.int64)
        if pattern == "zero":
            values = torch.zeros_like(index)
        elif pattern == "ramp":
            values = index * (seed * 2 + 1) + seed * 17
        else:
            values = index * (977 + seed * 13) + index.square() * 17 + seed * 131
        return (values % (maximum + 1)).to(torch.uint16)

    return (
        make_plane(width * height, 1).view(height, width),
        make_plane(chroma_width * height, 2).view(height, chroma_width),
        make_plane(chroma_width * height, 3).view(height, chroma_width),
    )


def _oracle_stream(width=257, height=130, bit_depth=10, quality=100, pattern="polynomial"):
    chroma_width = (width + 1) // 2
    planes = _source_planes(width, height, bit_depth, pattern)
    raw = b"".join(plane.numpy().tobytes() for plane in planes)
    with tempfile.TemporaryDirectory() as directory:
        source = Path(directory) / "source.yuv"
        encoded = Path(directory) / "frame.fvid"
        decoded = Path(directory) / "decoded.yuv"
        source.write_bytes(raw)
        subprocess.run(
            [
                str(BINARY),
                "encode-yuv422p16le-parallel-full-tile",
                str(source),
                str(encoded),
                str(width),
                str(height),
                "24/1",
                str(bit_depth),
                str(quality),
                "1",
                "256",
                "128",
            ],
            check=True,
        )
        subprocess.run([str(BINARY), "decode16", str(encoded), str(decoded), "1"], check=True)
        stream = encoded.read_bytes()
        oracle = torch.frombuffer(bytearray(decoded.read_bytes()), dtype=torch.uint16).clone()
    y_samples = width * height
    chroma_samples = chroma_width * height
    expected = (
        oracle[:y_samples].view(height, width),
        oracle[y_samples : y_samples + chroma_samples].view(height, chroma_width),
        oracle[y_samples + chroma_samples :].view(height, chroma_width),
    )
    return stream, expected


def test_encode_v5_is_byte_identical_to_rust_across_depth_quality_and_modes():
    selected_modes = set()
    for bit_depth in (10, 12, 16):
        for quality, pattern in ((90, "ramp"), (100, "zero"), (100, "polynomial")):
            width, height = 263, 133
            planes = _source_planes(width, height, bit_depth, pattern)
            rust_stream, _ = _oracle_stream(
                width=width,
                height=height,
                bit_depth=bit_depth,
                quality=quality,
                pattern=pattern,
            )
            cuda_stream = encode_v5(
                tuple(plane.cuda() for plane in planes),
                bit_depth=bit_depth,
                quality=quality,
            )
            assert cuda_stream.is_cuda
            assert cuda_stream.dtype == torch.uint8
            assert bytes(cuda_stream.cpu().numpy()) == rust_stream
            selected_modes.update(_shard_modes(rust_stream))
    assert 0 in selected_modes
    assert 18 in selected_modes
    assert any(1 <= mode <= 17 for mode in selected_modes)


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


def test_decode_v5_matches_rust_q100_from_dram_and_vram():
    assert torch.cuda.is_available()
    stream, expected = _oracle_stream()
    encoded_cpu = torch.frombuffer(bytearray(stream), dtype=torch.uint8)
    for encoded in (encoded_cpu, encoded_cpu.cuda()):
        for predictor in ("wavefront", "serial"):
            actual = decode_v5(encoded, predictor=predictor)
            assert len(actual) == 3
            for result, oracle in zip(actual, expected):
                assert result.is_cuda
                assert result.dtype == torch.uint16
                assert torch.equal(result.cpu(), oracle)


def test_decode_v5_matches_rust_across_depth_quality_and_entropy_modes():
    selected_modes = set()
    for bit_depth in (10, 12, 16):
        for quality, pattern in ((90, "ramp"), (100, "zero"), (100, "polynomial")):
            stream, expected = _oracle_stream(
                width=263,
                height=133,
                bit_depth=bit_depth,
                quality=quality,
                pattern=pattern,
            )
            selected_modes.update(_shard_modes(stream))
            encoded = torch.frombuffer(bytearray(stream), dtype=torch.uint8).cuda()
            for predictor in ("wavefront", "serial"):
                actual = decode_v5(encoded, predictor=predictor)
                for result, oracle in zip(actual, expected):
                    assert torch.equal(result.cpu(), oracle)
    assert 0 in selected_modes
    assert 18 in selected_modes
    assert any(1 <= mode <= 17 for mode in selected_modes)


def test_decode_v5_rejects_malformed_vram_directory_and_shards():
    stream, _ = _oracle_stream(width=263, height=133, quality=100)
    first_payload = struct.unpack_from("<Q", stream, 32 + 20)[0]
    mutations = []

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
        encoded = torch.frombuffer(malformed, dtype=torch.uint8).cuda()
        try:
            decode_v5(encoded)
        except RuntimeError:
            pass
        else:
            raise AssertionError("malformed CUDA-resident v5 stream was accepted")


if __name__ == "__main__":
    test_encode_v5_is_byte_identical_to_rust_across_depth_quality_and_modes()
    test_decode_v5_matches_rust_q100_from_dram_and_vram()
    test_decode_v5_matches_rust_across_depth_quality_and_entropy_modes()
    test_decode_v5_rejects_malformed_vram_directory_and_shards()
    print("CUDA v5 encode/decode matches Rust bytes/pixels across depths, qualities, and modes")
