"""Minimal in-memory ctypes bridge for libvship 5.0 metrics."""

from __future__ import annotations

import ctypes
import threading
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from typing import Any, Sequence


class _Subsampling(ctypes.Structure):
    _fields_ = [("subw", ctypes.c_int), ("subh", ctypes.c_int)]


class _Crop(ctypes.Structure):
    _fields_ = [
        ("top", ctypes.c_int), ("bottom", ctypes.c_int),
        ("left", ctypes.c_int), ("right", ctypes.c_int),
    ]


class _Colorspace(ctypes.Structure):
    _fields_ = [
        ("width", ctypes.c_int64), ("height", ctypes.c_int64),
        ("target_width", ctypes.c_int64), ("target_height", ctypes.c_int64),
        ("sample", ctypes.c_int), ("range", ctypes.c_int),
        ("subsampling", _Subsampling), ("chroma_location", ctypes.c_int),
        ("color_family", ctypes.c_int), ("yuv_matrix", ctypes.c_int),
        ("transfer_function", ctypes.c_int), ("primaries", ctypes.c_int),
        ("crop", _Crop),
    ]


class _Handler(ctypes.Structure):
    _fields_ = [("id", ctypes.c_uint)]


class _Version(ctypes.Structure):
    _fields_ = [
        ("major", ctypes.c_int), ("minor", ctypes.c_int),
        ("patch", ctypes.c_int), ("backend", ctypes.c_int),
    ]


class _ButteraugliScore(ctypes.Structure):
    _fields_ = [
        ("norm_q", ctypes.c_double),
        ("norm_3", ctypes.c_double),
        ("norm_inf", ctypes.c_double),
    ]


_Planes = ctypes.POINTER(ctypes.c_uint8) * 3
_Strides = ctypes.c_int64 * 3
_SAMPLE_TYPES = {8: 2, 10: 5, 16: 11}
_BACKENDS = {0: "HIP", 1: "CUDA", 2: "Vulkan"}


def libvship_version(library_path: Path) -> dict[str, Any]:
    library = ctypes.CDLL(str(library_path))
    library.Vship_GetVersion.argtypes = []
    library.Vship_GetVersion.restype = _Version
    version = library.Vship_GetVersion()
    return {
        "major": version.major, "minor": version.minor, "patch": version.patch,
        "backend": _BACKENDS.get(version.backend, f"unknown-{version.backend}"),
    }


def _colorspace(width: int, height: int, format_name: str, bit_depth: int) -> _Colorspace:
    if format_name == "yuv422":
        family, matrix, subsampling, chroma_location = 0, 1, _Subsampling(1, 0), 0
    elif format_name in ("rgb444", "gray"):
        family, matrix, subsampling, chroma_location = 1, 0, _Subsampling(0, 0), 1
    else:
        raise ValueError(f"unsupported libvship format {format_name!r}")
    return _Colorspace(
        width, height, -1, -1, _SAMPLE_TYPES[bit_depth], 1,
        subsampling, chroma_location, family, matrix, 1, 1,
        _Crop(0, 0, 0, 0),
    )


def _error_text(library: Any) -> str:
    buffer = ctypes.create_string_buffer(2048)
    library.Vship_GetDetailedLastError(buffer, len(buffer))
    return buffer.value.decode("utf-8", "replace")


class DirectVshipMetrics:
    """Persistent direct metric handlers for one geometry/format/depth."""

    def __init__(
        self, library_path: Path, width: int, height: int,
        format_name: str, bit_depth: int, gpu_id: int = 0, workers: int = 2,
    ) -> None:
        if workers < 1:
            raise ValueError("libvship workers must be positive")
        self.library = ctypes.CDLL(str(library_path))
        self.format_name = format_name
        self.bit_depth = bit_depth
        self.workers = workers
        self.metric_seconds = 0.0
        self.transfer_seconds = 0.0
        self.last_metric_seconds = 0.0
        self.last_transfer_seconds = 0.0
        self.frame_count = 0
        self._host_buffer_cache: dict[tuple[Any, ...], list[tuple[Any, ...]]] = {}
        self._closed = False
        self._configure_api()
        colorspace = _colorspace(width, height, format_name, bit_depth)
        self._ssim_handlers = [_Handler() for _ in range(workers)]
        self._butter_handlers = [_Handler() for _ in range(workers)]
        for handler in self._ssim_handlers:
            self._check(
                self.library.Vship_SSIMU2Init2(
                    ctypes.byref(handler), colorspace, colorspace, gpu_id,
                ),
                "SSIMULACRA2 initialization",
            )
        for handler in self._butter_handlers:
            self._check(
                self.library.Vship_ButteraugliInit2(
                    ctypes.byref(handler), colorspace, colorspace,
                    2, ctypes.c_float(1.0), gpu_id,
                ),
                "Butteraugli initialization",
            )
        self._ssim_executors = [ThreadPoolExecutor(max_workers=1) for _ in range(workers)]
        self._butter_executors = [ThreadPoolExecutor(max_workers=1) for _ in range(workers)]

    def _configure_api(self) -> None:
        lib = self.library
        lib.Vship_GetDetailedLastError.argtypes = [ctypes.c_char_p, ctypes.c_int]
        lib.Vship_GetDetailedLastError.restype = ctypes.c_int
        lib.Vship_SSIMU2Init2.argtypes = [
            ctypes.POINTER(_Handler), _Colorspace, _Colorspace, ctypes.c_int,
        ]
        lib.Vship_SSIMU2Init2.restype = ctypes.c_int
        lib.Vship_ComputeSSIMU2.argtypes = [
            _Handler, ctypes.POINTER(ctypes.c_double),
            _Planes, _Planes, _Strides, _Strides,
        ]
        lib.Vship_ComputeSSIMU2.restype = ctypes.c_int
        lib.Vship_SSIMU2Free.argtypes = [_Handler]
        lib.Vship_SSIMU2Free.restype = ctypes.c_int
        lib.Vship_ButteraugliInit2.argtypes = [
            ctypes.POINTER(_Handler), _Colorspace, _Colorspace,
            ctypes.c_int, ctypes.c_float, ctypes.c_int,
        ]
        lib.Vship_ButteraugliInit2.restype = ctypes.c_int
        lib.Vship_ComputeButteraugli.argtypes = [
            _Handler, ctypes.POINTER(_ButteraugliScore),
            ctypes.c_void_p, ctypes.c_int64,
            _Planes, _Planes, _Strides, _Strides,
        ]
        lib.Vship_ComputeButteraugli.restype = ctypes.c_int
        lib.Vship_ButteraugliFree.argtypes = [_Handler]
        lib.Vship_ButteraugliFree.restype = ctypes.c_int

    def _check(self, error: int, operation: str) -> None:
        if error:
            raise RuntimeError(f"libvship {operation} failed ({error}): {_error_text(self.library)}")

    def _copy_frames_to_host(
        self, frames: Sequence[Sequence[Any]], torch: Any, role: str,
    ) -> list[tuple[Any, ...]]:
        dtype = torch.uint8 if self.bit_depth == 8 else torch.uint16
        signature = (
            role, str(dtype),
            tuple(tuple(tuple(plane.shape) for plane in frame) for frame in frames),
        )
        buffers = self._host_buffer_cache.get(signature)
        if buffers is None:
            buffers = [
                tuple(
                    torch.empty(
                        plane.shape, dtype=dtype, device="cpu", pin_memory=True,
                    )
                    for plane in frame
                )
                for frame in frames
            ]
            self._host_buffer_cache[signature] = buffers
        for frame_buffers, frame in zip(buffers, frames):
            for host, plane in zip(frame_buffers, frame):
                host.copy_(plane, non_blocking=True)
        return buffers

    def _arguments(self, frame: Sequence[Any]) -> tuple[_Planes, _Strides]:
        planes = frame if self.format_name != "gray" else (frame[0],) * 3
        pointers = _Planes(*(
            ctypes.cast(plane.data_ptr(), ctypes.POINTER(ctypes.c_uint8))
            for plane in planes
        ))
        strides = _Strides(*(
            plane.stride(0) * plane.element_size() for plane in planes
        ))
        return pointers, strides

    def _ssim(self, handler: _Handler, source: Sequence[Any], distorted: Sequence[Any]) -> float:
        source_planes, source_strides = self._arguments(source)
        distorted_planes, distorted_strides = self._arguments(distorted)
        score = ctypes.c_double()
        self._check(
            self.library.Vship_ComputeSSIMU2(
                handler, ctypes.byref(score), source_planes, distorted_planes,
                source_strides, distorted_strides,
            ),
            "SSIMULACRA2 computation",
        )
        return score.value

    def _butter(
        self, handler: _Handler, source: Sequence[Any], distorted: Sequence[Any],
    ) -> tuple[float, float, float]:
        source_planes, source_strides = self._arguments(source)
        distorted_planes, distorted_strides = self._arguments(distorted)
        score = _ButteraugliScore()
        self._check(
            self.library.Vship_ComputeButteraugli(
                handler, ctypes.byref(score), None, 0,
                source_planes, distorted_planes, source_strides, distorted_strides,
            ),
            "Butteraugli computation",
        )
        return score.norm_q, score.norm_3, score.norm_inf

    def evaluate(
        self, source_frames: Sequence[Sequence[Any]],
        distorted_frames: Sequence[Sequence[Any]], torch: Any,
    ) -> tuple[list[float], list[float]]:
        if self._closed:
            raise RuntimeError("libvship metric pool is closed")
        if len(source_frames) != len(distorted_frames):
            raise ValueError("source and distorted frame counts differ")
        transfer_started = time.perf_counter()
        source_host = self._copy_frames_to_host(source_frames, torch, "source")
        distorted_host = self._copy_frames_to_host(
            distorted_frames, torch, "distorted",
        )
        torch.cuda.synchronize()
        self.last_transfer_seconds = time.perf_counter() - transfer_started
        self.transfer_seconds += self.last_transfer_seconds
        started = time.perf_counter()
        ssim_futures = []
        butter_futures = []
        for number, (source, distorted) in enumerate(zip(source_host, distorted_host)):
            worker = number % self.workers
            ssim_futures.append(self._ssim_executors[worker].submit(
                self._ssim, self._ssim_handlers[worker], source, distorted,
            ))
            butter_futures.append(self._butter_executors[worker].submit(
                self._butter, self._butter_handlers[worker], source, distorted,
            ))
        ssim = [future.result() for future in ssim_futures]
        butter_norms = [future.result() for future in butter_futures]
        self.last_metric_seconds = time.perf_counter() - started
        self.metric_seconds += self.last_metric_seconds
        self.frame_count += len(source_frames)
        return ssim, [max(norms) for norms in butter_norms]

    def close(self) -> None:
        if self._closed:
            return
        for executor in self._ssim_executors + self._butter_executors:
            executor.shutdown()
        for handler in self._ssim_handlers:
            self._check(self.library.Vship_SSIMU2Free(handler), "SSIMULACRA2 free")
        for handler in self._butter_handlers:
            self._check(self.library.Vship_ButteraugliFree(handler), "Butteraugli free")
        self._closed = True

    def __enter__(self) -> "DirectVshipMetrics":
        return self

    def __exit__(self, *_: Any) -> None:
        self.close()
