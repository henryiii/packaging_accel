import pytest

from packaging_accel import split_version


BENCHMARK_CASES = [
    "1.2.3",
    "1.2.3.4.5.6",
    "1.2.3.4.5.6.7.8.9.10.11.12",
]


def py_baseline(version: str) -> tuple[int, ...]:
    return tuple(map(int, version.split(".")))


@pytest.mark.parametrize("version", BENCHMARK_CASES, ids=BENCHMARK_CASES)
@pytest.mark.benchmark(group="split-version")
def test_benchmark_split_version(benchmark: pytest.BenchmarkFixture, version: str) -> None:
    result = benchmark(split_version, version)
    assert result == py_baseline(version)


@pytest.mark.parametrize("version", BENCHMARK_CASES, ids=BENCHMARK_CASES)
@pytest.mark.benchmark(group="split-version")
def test_benchmark_split_version_python_baseline(
    benchmark: pytest.BenchmarkFixture, version: str
) -> None:
    result = benchmark(py_baseline, version)
    assert result