"""Focused tests for deterministic corpus sample conversion."""
import numpy as np
from scripts.extract_corpus import MATRIX, REJECTION_CASES, quantize, yuv422


def test_quantize_retains_uint16_container_and_range():
    source = np.array([0, 32768, 65535], dtype=np.uint16)
    result = quantize(source, 10)
    assert result.dtype == np.dtype("<u2")
    assert result.tolist() == [0, 512, 1023]


def test_yuv422_shapes_and_neutral_chroma():
    rgb = np.full((2, 4, 3), 32768, dtype=np.uint16)
    y, cb, cr = yuv422(rgb)
    assert y.shape == (2, 4)
    assert cb.shape == cr.shape == (2, 2)
    assert np.all(y == 32768)
    assert np.all(cb == 32768)
    assert np.all(cr == 32768)


def test_rejection_cases_cover_matrix_once():
    assert len(REJECTION_CASES) == len(MATRIX)
    assert {(fmt, depth) for _, fmt, depth in REJECTION_CASES} == set(MATRIX)
