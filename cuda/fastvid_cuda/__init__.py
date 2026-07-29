"""PyTorch bindings for the experimental Fastvid CUDA implementation."""

import torch

from . import _C


def encode(
    planes,
    *,
    layout=None,
    bit_depth=10,
    quality=90,
    frame_rate=(24, 1),
    tile_size=(256, 128),
):
    """Encode one CUDA uint16 grayscale or planar YUV 4:2:2 frame.

    ``planes`` is ``(Y,)`` or ``(Y, Cb, Cr)``. The returned version-1
    bitstream is a one-dimensional CUDA uint8 tensor. The API is deliberately
    frame-oriented; the public ``fastvid`` package handles independent batches.
    """

    if isinstance(planes, torch.Tensor):
        planes = (planes,)
    else:
        planes = tuple(planes)
    if layout is None:
        layout = 0 if len(planes) == 1 else (
            1 if planes[1].shape[1] != planes[0].shape[1] else 2
        )
    elif isinstance(layout, str):
        layout = {"gray": 0, "yuv422": 1, "rgb444": 2}[layout]
    fps_numerator, fps_denominator = frame_rate
    tile_width, tile_height = tile_size
    return _C.encode(
        list(planes),
        layout,
        bit_depth,
        quality,
        fps_numerator,
        fps_denominator,
        tile_width,
        tile_height,
    )


def decode(encoded, predictor="wavefront"):
    """Decode one Fastvid version-1 frame to CUDA plane tensors.

    ``encoded`` is a one-dimensional uint8 tensor in CPU DRAM or CUDA VRAM.
    The result is ``(Y,)`` for grayscale or ``(Y, Cb, Cr)`` for YUV 4:2:2.
    Every returned plane is a contiguous CUDA uint16 tensor.
    """

    if predictor not in ("wavefront", "serial"):
        raise ValueError("predictor must be 'wavefront' or 'serial'")
    return tuple(_C.decode(encoded, predictor == "wavefront"))


__all__ = ["decode", "encode"]
