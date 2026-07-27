"""PyTorch bindings for the experimental Fastvid CUDA implementation."""

from . import _C


def decode_v5(encoded, predictor="wavefront"):
    """Decode one Rust-compatible Fastvid v5 frame to CUDA plane tensors.

    ``encoded`` is a one-dimensional uint8 tensor in CPU DRAM or CUDA VRAM.
    The result is ``(Y,)`` for grayscale or ``(Y, Cb, Cr)`` for YUV 4:2:2.
    Every returned plane is a contiguous CUDA uint16 tensor.
    """

    if predictor not in ("wavefront", "serial"):
        raise ValueError("predictor must be 'wavefront' or 'serial'")
    return tuple(_C.decode_v5(encoded, predictor == "wavefront"))


__all__ = ["decode_v5"]
