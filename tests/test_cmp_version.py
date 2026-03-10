from __future__ import annotations

import pytest

from packaging.version import Version

from packaging_accel import cmp_version, parse_version


def _expected_cmp(left: str, right: str) -> int:
	left_v = Version(left)
	right_v = Version(right)
	return (left_v > right_v) - (left_v < right_v)


@pytest.mark.parametrize(
	("left", "right"),
	[
		("1", "1.0.0"),
		("1.0dev1", "1.0a1"),
		("1.0a1", "1.0a1.dev1"),
		("1.0a1", "1.0b1"),
		("1.0b1", "1.0rc1"),
		("1.0rc1", "1.0"),
		("1.0", "1.0post1"),
		("1.0post1dev1", "1.0post1"),
		("1!1.0", "2.0"),
		("1.0+abc", "1.0"),
		("1.0+abc.1", "1.0+abc.2"),
		("1.0+1", "1.0+abc"),
		("1.0+abc", "1.0+abc.1"),
		("2.0", "1!1.0"),
	],
)
def test_cmp_version_matches_packaging(left: str, right: str) -> None:
	left_tuple = parse_version(left)
	right_tuple = parse_version(right)
	assert cmp_version(left_tuple, right_tuple) == _expected_cmp(left, right)


def test_cmp_version_rejects_unknown_pre_tag() -> None:
	with pytest.raises(ValueError):
		cmp_version((None, (1,), ("preview", 1), None, None, None), parse_version("1"))
