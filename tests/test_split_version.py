import pytest

from packaging_accel import split_version


def test_returns_variable_length_tuple() -> None:
    assert split_version("1.2.3.4.5") == (1, 2, 3, 4, 5)

def test_non_numeric_parts_raise_error() -> None:
    with pytest.raises(ValueError):
        split_version("2.a.7")


def test_empty_segment_raises_error() -> None:
    with pytest.raises(ValueError):
        split_version("1..3")


def test_allows_unicode_whitespace_only() -> None:
    assert split_version("\u2003 1.2.3 \u00a0") == (1, 2, 3)


def test_rejects_non_whitespace_unicode() -> None:
    with pytest.raises(ValueError):
        split_version("1.2.3\u03b1")
