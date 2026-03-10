def split_version(version: str) -> tuple[int, ...]: ...


def parse_version(
	version: str,
) -> tuple[
	int | None,
	tuple[int, ...] | None,
	tuple[str, int] | None,
	int | None,
	int | None,
	str | None,
]: ...


def cmp_version(
	left: tuple[
		int | None,
		tuple[int, ...],
		tuple[str, int] | None,
		int | None,
		int | None,
		str | None,
	],
	right: tuple[
		int | None,
		tuple[int, ...],
		tuple[str, int] | None,
		int | None,
		int | None,
		str | None,
	],
) -> int: ...
