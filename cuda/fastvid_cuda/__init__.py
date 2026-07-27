"""PyTorch bindings for the experimental Fastvid CUDA implementation."""

from . import _C


def encode_v5(
    planes,
    *,
    bit_depth=10,
    quality=90,
    frame_rate=(24, 1),
    tile_size=(256, 128),
):
    """Encode one CUDA uint16 grayscale or planar YUV 4:2:2 frame.

    ``planes`` is ``(Y,)`` or ``(Y, Cb, Cr)``. The returned canonical v5
    byte stream is a one-dimensional CUDA uint8 tensor. This initial API is
    deliberately frame-oriented; video batching and RGB/4:4:4 conversion are
    separate format/API milestones.
    """

    if isinstance(planes, torch.Tensor):
        planes = (planes,)
    else:
        planes = tuple(planes)
    fps_numerator, fps_denominator = frame_rate
    tile_width, tile_height = tile_size
    return _C.encode_v5(
        list(planes),
        bit_depth,
        quality,
        fps_numerator,
        fps_denominator,
        tile_width,
        tile_height,
    )


def decode_v5(encoded, predictor="wavefront"):
    """Decode one Rust-compatible Fastvid v5 frame to CUDA plane tensors.

    ``encoded`` is a one-dimensional uint8 tensor in CPU DRAM or CUDA VRAM.
    The result is ``(Y,)`` for grayscale or ``(Y, Cb, Cr)`` for YUV 4:2:2.
    Every returned plane is a contiguous CUDA uint16 tensor.
    """

    if predictor not in ("wavefront", "serial"):
        raise ValueError("predictor must be 'wavefront' or 'serial'")
    return tuple(_C.decode_v5(encoded, predictor == "wavefront"))


import torch


__all__ = ["decode_v5", "encode_v5"]
