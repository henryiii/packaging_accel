use pyo3::prelude::*;

/// A Python module implemented in Rust. The name of this module must match
/// the `lib.name` setting in the `Cargo.toml`, else Python will not be able to
/// import the module.
#[pymodule]
mod _core {
    use pyo3::{exceptions::PyValueError, prelude::*, types::PyTuple};
    use smallvec::SmallVec;

    #[derive(Clone, Copy)]
    enum PreTag {
        A,
        B,
        Rc,
    }

    impl PreTag {
        fn as_str(self) -> &'static str {
            match self {
                Self::A => "a",
                Self::B => "b",
                Self::Rc => "rc",
            }
        }
    }

    type ParseVersionResult = (
        Option<i64>,
        Py<PyTuple>,
        Option<(String, u64)>,
        Option<u64>,
        Option<u64>,
        Option<String>,
    );

    const PRE_LABELS: [(&[u8], PreTag); 8] = [
        (b"preview", PreTag::Rc),
        (b"alpha", PreTag::A),
        (b"beta", PreTag::B),
        (b"pre", PreTag::Rc),
        (b"rc", PreTag::Rc),
        (b"a", PreTag::A),
        (b"b", PreTag::B),
        (b"c", PreTag::Rc),
    ];

    const POST_LABELS: [(&[u8], &str); 3] = [(b"post", "post"), (b"rev", "post"), (b"r", "post")];

    fn starts_with_ci(bytes: &[u8], i: usize, pattern: &[u8]) -> bool {
        if i + pattern.len() > bytes.len() {
            return false;
        }
        bytes[i..i + pattern.len()]
            .iter()
            .zip(pattern.iter())
            .all(|(left, right)| left.to_ascii_lowercase() == *right)
    }

    fn parse_digits(bytes: &[u8], mut i: usize) -> Option<(u64, usize)> {
        let start = i;
        let mut value = 0u64;

        while i < bytes.len() {
            let b = bytes[i];
            if !b.is_ascii_digit() {
                break;
            }
            let digit = (b - b'0') as u64;
            value = value.checked_mul(10)?.checked_add(digit)?;
            i += 1;
        }

        if i == start {
            return None;
        }

        Some((value, i))
    }

    fn consume_optional_sep(bytes: &[u8], i: usize) -> usize {
        if i < bytes.len() {
            let b = bytes[i];
            if b == b'.' || b == b'_' || b == b'-' {
                return i + 1;
            }
        }
        i
    }

    fn parse_label<T: Copy>(bytes: &[u8], i: usize, labels: &[(&[u8], T)]) -> Option<(T, usize)> {
        labels.iter().find_map(|(label, canonical)| {
            starts_with_ci(bytes, i, label).then_some((*canonical, i + label.len()))
        })
    }

    fn parse_optional_number(bytes: &[u8], i: usize) -> (u64, usize) {
        let n_i = consume_optional_sep(bytes, i);
        if let Some((n, parsed_i)) = parse_digits(bytes, n_i) {
            (n, parsed_i)
        } else {
            (0, n_i)
        }
    }

    #[pyfunction]
    fn split_version(py: Python, version: &str) -> PyResult<Py<PyTuple>> {
        let version = version.trim();
        if version.is_empty() {
            return Err(PyValueError::new_err(
                "version segments must be valid (64-bit) integers",
            ));
        }

        let mut parts = Vec::with_capacity(version.as_bytes().iter().filter(|&&b| b == b'.').count() + 1);
        for segment in version.split('.') {
            let value = segment.parse::<u64>().map_err(|_| {
                PyValueError::new_err(
                    "version segments must be valid (64-bit) integers",
                )
            })?;
            parts.push(value);
        }
        Ok(PyTuple::new(py, parts)?.into())
    }

    #[pyfunction]
    fn parse_version(
        py: Python,
        version: &str,
    ) -> PyResult<ParseVersionResult> {
        let version = version.trim();
        let bytes = version.as_bytes();
        let mut i = 0usize;

        if i < bytes.len() && (bytes[i] == b'v' || bytes[i] == b'V') {
            i = 1;
        }

        let mut epoch = None;
        if let Some((epoch_u64, next_i)) = parse_digits(bytes, i) {
            if next_i < bytes.len() && bytes[next_i] == b'!' {
                let parsed_epoch = i64::try_from(epoch_u64).map_err(|_| {
                    PyValueError::new_err("epoch is too large to fit into a 64-bit signed integer")
                })?;
                epoch = Some(parsed_epoch);
                i = next_i + 1;
            }
        }

        let mut release: SmallVec<[u64; 4]> = SmallVec::new();
        let (first_release, mut next_i) = parse_digits(bytes, i).ok_or_else(|| {
            PyValueError::new_err("invalid version: expected release segment")
        })?;
        release.push(first_release);
        i = next_i;

        while i < bytes.len() && bytes[i] == b'.' {
            next_i = i + 1;
            let (part, parsed_i) = parse_digits(bytes, next_i).ok_or_else(|| {
                PyValueError::new_err("invalid version: release segments must be numeric")
            })?;
            release.push(part);
            i = parsed_i;
        }

        let mut pre: Option<(PreTag, u64)> = None;
        let pre_start = i;
        let pre_with_sep = consume_optional_sep(bytes, i);
        if let Some((label, label_end)) = parse_label(bytes, pre_with_sep, &PRE_LABELS) {
            let (num, n_i) = parse_optional_number(bytes, label_end);
            pre = Some((label, num));
            i = n_i;
        } else {
            i = pre_start;
        }

        let mut post = None;
        let post_start = i;
        if i < bytes.len() && bytes[i] == b'-' {
            if let Some((n, parsed_i)) = parse_digits(bytes, i + 1) {
                post = Some(n);
                i = parsed_i;
            }
        }
        if post.is_none() {
            let post_with_sep = consume_optional_sep(bytes, post_start);
            if let Some((_label, label_end)) = parse_label(bytes, post_with_sep, &POST_LABELS) {
                let (num, n_i) = parse_optional_number(bytes, label_end);
                post = Some(num);
                i = n_i;
            }
        }

        let mut dev = None;
        let dev_start = i;
        let dev_with_sep = consume_optional_sep(bytes, i);
        if starts_with_ci(bytes, dev_with_sep, b"dev") {
            let (num, n_i) = parse_optional_number(bytes, dev_with_sep + 3);
            dev = Some(num);
            i = n_i;
        } else {
            i = dev_start;
        }

        let mut local = None;
        if i < bytes.len() && bytes[i] == b'+' {
            i += 1;
            let local_start = i;
            let mut seg_len = 0usize;
            while i < bytes.len() {
                let b = bytes[i];
                if b.is_ascii_alphanumeric() {
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
            local = Some(version[local_start..i].to_ascii_lowercase());
        }

        if i != bytes.len() {
            return Err(PyValueError::new_err("invalid version"));
        }

        Ok((
            epoch,
            PyTuple::new(py, release.iter().copied())?.into(),
            pre.map(|(tag, n)| (tag.as_str().to_string(), n)),
            post,
            dev,
            local,
        ))
    }
}
