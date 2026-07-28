"""Public Python API for the CUDA Fastvid codec."""

from __future__ import annotations

import struct
from collections.abc import Sequence

import torch

from fastvid_cuda import decode_v5, encode_v5

_LAYOUT_TO_FORMAT = {0: "gray", 1: "yuv422", 2: "rgb444"}


def _is_frame(value) -> bool:
    return (
        isinstance(value, (tuple, list))
        and bool(value)
        and all(isinstance(plane, torch.Tensor) for plane in value)
    )


def encode(
    frame,
    *,
    format: str,
    bit_depth: int,
    quality: int = 90,
    tile_size: tuple[int, int] = (256, 128),
):
    """Encode one frame or a sequence of independent frames."""

    if _is_frame(frame):
        return encode_v5(
            frame,
            layout=format,
            bit_depth=bit_depth,
            quality=quality,
            tile_size=tile_size,
        )
    if not isinstance(frame, Sequence):
        raise TypeError("frame must be planar tensors or a sequence of frames")
    return [
        encode_v5(
            item,
            layout=format,
            bit_depth=bit_depth,
            quality=quality,
            tile_size=tile_size,
        )
        for item in frame
    ]


def decode(encoded):
    """Decode one independent stream or a sequence of streams."""

    if isinstance(encoded, torch.Tensor):
        return decode_v5(encoded)
    if not isinstance(encoded, Sequence):
        raise TypeError("encoded must be a byte tensor or a sequence of byte tensors")
    return [decode_v5(stream) for stream in encoded]


def inspect(encoded) -> dict[str, int | str]:
    """Return self-described frame metadata and exact overhead accounting."""

    if not isinstance(encoded, torch.Tensor) or encoded.dtype != torch.uint8 or encoded.ndim != 1:
        raise TypeError("encoded must be a one-dimensional uint8 tensor")
    header = bytes(encoded[:32].cpu().numpy())
    if len(header) != 32 or header[:4] != b"FVID" or header[4] != 5:
        raise ValueError("not a Fastvid v5 stream")
    layout = header[5]
    if layout not in _LAYOUT_TO_FORMAT:
        raise ValueError("unknown pixel layout")
    width, height = struct.unpack_from("<II", header, 8)
    tile_count = struct.unpack_from("<I", header, 28)[0]
    return {
        "width": width,
        "height": height,
        "format": _LAYOUT_TO_FORMAT[layout],
        "bit_depth": header[7] + 8,
        "frame_count": 1,
        "metadata_bytes": 32 + tile_count * 32,
        "container_overhead_bytes": 0,
    }


__all__ = ["decode", "encode", "inspect"]
