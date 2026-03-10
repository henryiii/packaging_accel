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
