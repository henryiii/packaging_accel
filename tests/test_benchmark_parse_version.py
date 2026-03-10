import pytest

from packaging.version import Version

from packaging_accel import parse_version


BENCHMARK_CASES = [
    "1.2.3",
    "1!1.2.3rc4.post5.dev6+abc.1",
    "2026.3.10.1",
]


@pytest.mark.parametrize("version", BENCHMARK_CASES, ids=BENCHMARK_CASES)
@pytest.mark.benchmark(group="parse-version")
def test_benchmark_parse_version(benchmark: pytest.BenchmarkFixture, version: str) -> None:
    result = benchmark(parse_version, version)
    assert result[1] is not None


@pytest.mark.parametrize("version", BENCHMARK_CASES, ids=BENCHMARK_CASES)
@pytest.mark.benchmark(group="parse-version")
def test_benchmark_packaging_version(benchmark: pytest.BenchmarkFixture, version: str) -> None:
    result = benchmark(Version, version)
    assert result.release