import pytest

from packaging_accel import parse_version
from packaging.version import Version


def test_parse_version_fields() -> None:
    ver_str = "1!1.2.3rc4.post5.dev6+abc.1"
    epoch, release, pre, post, dev, local = parse_version(ver_str)
    assert epoch == 1
    assert release == (1, 2, 3)
    assert pre == ("rc", 4)
    assert post == 5
    assert dev == 6
    assert local == "abc.1"

    ver = Version(ver_str)
    assert epoch == ver.epoch
    assert release == ver.release
    assert pre == ver.pre
    assert post == ver.post
    assert dev == ver.dev
    assert local == ver.local


def test_parse_version_invalid_raises_value_error() -> None:
    with pytest.raises(ValueError):
        parse_version("1..2")


def test_parse_version_allows_unicode_whitespace_only() -> None:
    epoch, release, pre, post, dev, local = parse_version("\u2003 1.2.3rc1 \u00a0")
    assert epoch is None
    assert release == (1, 2, 3)
    assert pre == ("rc", 1)
    assert post is None
    assert dev is None
    assert local is None


def test_parse_version_rejects_non_whitespace_unicode() -> None:
    with pytest.raises(ValueError):
        parse_version("1.2.3\u03b1")


@pytest.mark.parametrize(
    ("version", "expected_pre", "expected_post", "expected_local"),
    [
        ("1ALPHA2", ("a", 2), None, None),
        ("1Beta3", ("b", 3), None, None),
        ("1PRE2", ("rc", 2), None, None),
        ("1Preview4", ("rc", 4), None, None),
        ("1C5", ("rc", 5), None, None),
        ("1REV2", None, 2, None),
        ("1R3", None, 3, None),
        ("1+ABC.Def", None, None, "abc.def"),
    ],
)
def test_parse_version_normalizes_results(
    version: str,
    expected_pre: tuple[str, int] | None,
    expected_post: int | None,
    expected_local: str | None,
) -> None:
    _, _, pre, post, _, local = parse_version(version)

    assert pre == expected_pre
    assert post == expected_post
    assert local == expected_local

    ver = Version(version)
    assert pre == ver.pre
    assert post == ver.post
    assert local == ver.local