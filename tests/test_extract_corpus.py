"""Focused tests for deterministic corpus sample conversion."""
from collections import Counter

import numpy as np

from scripts.extract_corpus import (
    MATRIX, REJECTION_CASES, extraction_case_map, quantize, rows,
    stratified_cases, yuv422,
)


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


def test_strata_assign_every_source_once_per_format_and_balance_depths():
    source_rows = rows()
    assigned = [case for index in range(len(source_rows)) for case in stratified_cases(index)]
    counts = Counter(assigned)
    assert len(source_rows) == 120
    assert counts == {
        ("yuv422", 8): 40, ("yuv422", 10): 40, ("yuv422", 16): 40,
        ("rgb444", 10): 60, ("rgb444", 16): 60,
        ("gray", 8): 40, ("gray", 10): 40, ("gray", 16): 40,
    }
    for index in range(len(source_rows)):
        assert {fmt for fmt, _ in stratified_cases(index)} == {
            "yuv422", "rgb444", "gray",
        }


def test_extraction_union_retains_rejection_and_performance_cases():
    source_rows = rows()
    case_map = extraction_case_map(source_rows)
    for item_id, fmt, depth in REJECTION_CASES:
        assert (fmt, depth) in case_map[item_id]
    performance_ids = [row["id"] for row in source_rows if not row["ai"]][:24]
    for item_id in performance_ids:
        assert ("yuv422", 10) in case_map[item_id]
        assert ("rgb444", 10) in case_map[item_id]
    counts = Counter(case for cases in case_map.values() for case in cases)
    assert sum(counts.values()) == 394
    assert set(counts) == set(MATRIX)
    assert min(counts.values()) >= 40
    assert case_map == extraction_case_map(source_rows)
