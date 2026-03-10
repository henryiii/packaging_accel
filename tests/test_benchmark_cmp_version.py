import pytest

from packaging.version import Version

from packaging_accel import cmp_version, parse_version


BENCHMARK_CASES = [
    ("1.0dev1", "1.0a1"),
    ("1.0a1", "1.0b1"),
    ("1.0rc1", "1.0"),
    ("1.0", "1.0post1"),
    ("1.0post1dev1", "1.0post1"),
    ("1.0+abc.1", "1.0+abc.2"),
    ("1.0+1", "1.0+abc"),
    ("1!1.0", "2.0"),
]


def _cmp_versions_no_cache(left: Version, right: Version) -> int:
    left._key_cache = None
    right._key_cache = None
    return (left > right) - (left < right)

def _cmp_versions(left: Version, right: Version) -> int:
    return (left > right) - (left < right)


@pytest.mark.parametrize(
    ("left", "right"),
    BENCHMARK_CASES,
    ids=[f"{left}__{right}" for left, right in BENCHMARK_CASES],
)
@pytest.mark.benchmark(group="cmp-version")
def test_benchmark_cmp_version(benchmark: pytest.BenchmarkFixture, left: str, right: str) -> None:
    left_tuple = parse_version(left)
    right_tuple = parse_version(right)
    expected = _cmp_versions(Version(left), Version(right))

    result = benchmark(cmp_version, left_tuple, right_tuple)
    assert result == expected


@pytest.mark.parametrize(
    ("left", "right"),
    BENCHMARK_CASES,
    ids=[f"{left}__{right}" for left, right in BENCHMARK_CASES],
)
@pytest.mark.benchmark(group="cmp-version")
def test_benchmark_packaging_version_compare_cold(
    benchmark: pytest.BenchmarkFixture, left: str, right: str
) -> None:
    left_version = Version(left)
    right_version = Version(right)

    result = benchmark(_cmp_versions_no_cache, left_version, right_version)
    assert result in (-1, 0, 1)


@pytest.mark.parametrize(
    ("left", "right"),
    BENCHMARK_CASES,
    ids=[f"{left}__{right}" for left, right in BENCHMARK_CASES],
)
@pytest.mark.benchmark(group="cmp-version")
def test_benchmark_packaging_version_compare_hot(
    benchmark: pytest.BenchmarkFixture, left: str, right: str
) -> None:
    left_version = Version(left)
    right_version = Version(right)
    left_version._key
    right_version._key

    result = benchmark(_cmp_versions, left_version, right_version)
    assert result in (-1, 0, 1)