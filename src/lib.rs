use pyo3::prelude::*;

/// A Python module implemented in Rust. The name of this module must match
/// the `lib.name` setting in the `Cargo.toml`, else Python will not be able to
/// import the module.
#[pymodule]
mod _core {
    use pyo3::{exceptions::PyValueError, prelude::*, types::PyTuple};

    const PRE_LABELS: [(&str, &str); 8] = [
        ("preview", "rc"),
        ("alpha", "a"),
        ("beta", "b"),
        ("pre", "rc"),
        ("rc", "rc"),
        ("a", "a"),
        ("b", "b"),
        ("c", "rc"),
    ];

    const POST_LABELS: [(&str, &str); 3] = [("post", "post"), ("rev", "post"), ("r", "post")];

    fn parse_u64_component(input: &str) -> Option<u64> {
        if input.is_empty() || !input.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        input.parse::<u64>().ok()
    }

    fn parse_digits(s: &str, mut i: usize) -> Option<(u64, usize)> {
        let start = i;
        while i < s.len() && s.as_bytes()[i].is_ascii_digit() {
            i += 1;
        }
        if i == start {
            return None;
        }
        parse_u64_component(&s[start..i]).map(|value| (value, i))
    }

    fn consume_optional_sep(s: &str, i: usize) -> usize {
        if i < s.len() {
            let b = s.as_bytes()[i];
            if b == b'.' || b == b'_' || b == b'-' {
                return i + 1;
            }
        }
        i
    }

    fn parse_label<'a>(s: &'a str, i: usize, labels: &[(&'a str, &'a str)]) -> Option<(&'a str, usize)> {
        labels
            .iter()
            .find_map(|(label, canonical)| s[i..].starts_with(label).then_some((*canonical, i + label.len())))
    }

    #[pyfunction]
    fn split_version(py: Python, version: String) -> PyResult<Py<PyTuple>> {
        let parts: Vec<u64> = version
            .split('.')
            .map(|s| s.parse::<u64>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| {
                PyValueError::new_err(
                    "version segments must be valid (64-bit) integers",
                )
            })?;
        Ok(PyTuple::new(py, parts)?.into())
    }

    #[pyfunction]
    fn parse_version(
        py: Python,
        version: String,
    ) -> PyResult<(
        Option<i64>,
        Py<PyTuple>,
        Option<(String, u64)>,
        Option<u64>,
        Option<u64>,
        Option<String>,
    )> {
        let s = version.to_ascii_lowercase();
        let mut i = 0usize;

        if s.starts_with('v') {
            i = 1;
        }

        let mut epoch = None;
        if let Some((epoch_u64, next_i)) = parse_digits(&s, i) {
            if next_i < s.len() && s.as_bytes()[next_i] == b'!' {
                let parsed_epoch = i64::try_from(epoch_u64).map_err(|_| {
                    PyValueError::new_err("epoch is too large to fit into a 64-bit signed integer")
                })?;
                epoch = Some(parsed_epoch);
                i = next_i + 1;
            }
        }

        let mut release = Vec::new();
        let (first_release, mut next_i) = parse_digits(&s, i).ok_or_else(|| {
            PyValueError::new_err("invalid version: expected release segment")
        })?;
        release.push(first_release);
        i = next_i;

        while i < s.len() && s.as_bytes()[i] == b'.' {
            next_i = i + 1;
            let (part, parsed_i) = parse_digits(&s, next_i).ok_or_else(|| {
                PyValueError::new_err("invalid version: release segments must be numeric")
            })?;
            release.push(part);
            i = parsed_i;
        }

        let mut pre = None;
        let pre_start = i;
        let pre_with_sep = consume_optional_sep(&s, i);
        if let Some((label, label_end)) = parse_label(&s, pre_with_sep, &PRE_LABELS) {
            let mut n_i = consume_optional_sep(&s, label_end);
            let num = if let Some((n, parsed_i)) = parse_digits(&s, n_i) {
                n_i = parsed_i;
                n
            } else {
                0
            };
            pre = Some((label.to_string(), num));
            i = n_i;
        } else {
            i = pre_start;
        }

        let mut post = None;
        let post_start = i;
        if i < s.len() && s.as_bytes()[i] == b'-' {
            if let Some((n, parsed_i)) = parse_digits(&s, i + 1) {
                post = Some(n);
                i = parsed_i;
            }
        }
        if post.is_none() {
            let post_with_sep = consume_optional_sep(&s, post_start);
            if let Some((_label, label_end)) = parse_label(&s, post_with_sep, &POST_LABELS) {
                let mut n_i = consume_optional_sep(&s, label_end);
                let num = if let Some((n, parsed_i)) = parse_digits(&s, n_i) {
                    n_i = parsed_i;
                    n
                } else {
                    0
                };
                post = Some(num);
                i = n_i;
            }
        }

        let mut dev = None;
        let dev_start = i;
        let dev_with_sep = consume_optional_sep(&s, i);
        if s[dev_with_sep..].starts_with("dev") {
            let mut n_i = consume_optional_sep(&s, dev_with_sep + 3);
            let num = if let Some((n, parsed_i)) = parse_digits(&s, n_i) {
                n_i = parsed_i;
                n
            } else {
                0
            };
            dev = Some(num);
            i = n_i;
        } else {
            i = dev_start;
        }

        let mut local = None;
        if i < s.len() && s.as_bytes()[i] == b'+' {
            i += 1;
            let local_start = i;
            let mut seg_len = 0usize;
            while i < s.len() {
                let b = s.as_bytes()[i];
                if b.is_ascii_lowercase() || b.is_ascii_digit() {
                    seg_len += 1;
                    i += 1;
                    continue;
                }
                if b == b'.' || b == b'_' || b == b'-' {
                    if seg_len == 0 {
                        return Err(PyValueError::new_err(
                            "invalid version: local segment cannot be empty",
                        ));
                    }
                    seg_len = 0;
                    i += 1;
                    continue;
                }
                break;
            }
            if seg_len == 0 || i == local_start {
                return Err(PyValueError::new_err(
                    "invalid version: local version is malformed",
                ));
            }
            local = Some(s[local_start..i].to_string());
        }

        if i != s.len() {
            return Err(PyValueError::new_err("invalid version"));
        }

        Ok((
            epoch,
            PyTuple::new(py, release)?.into(),
            pre,
            post,
            dev,
            local,
        ))
    }
}
