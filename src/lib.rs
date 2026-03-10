use pyo3::prelude::*;

/// A Python module implemented in Rust. The name of this module must match
/// the `lib.name` setting in the `Cargo.toml`, else Python will not be able to
/// import the module.
#[pymodule]
mod _core {
    use std::cmp::Ordering;

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

    type VersionTuple = (
        Option<i64>,
        Vec<u64>,
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

    fn compare_release(left: &[u64], right: &[u64]) -> Ordering {
        let mut i = 0usize;
        while i < left.len() && i < right.len() {
            let l = left[i];
            let r = right[i];
            match l.cmp(&r) {
                Ordering::Equal => {}
                non_eq => return non_eq,
            }
            i += 1;
        }

        while i < left.len() {
            match left[i].cmp(&0) {
                Ordering::Equal => {}
                non_eq => return non_eq,
            }
            i += 1;
        }

        while i < right.len() {
            match 0.cmp(&right[i]) {
                Ordering::Equal => {}
                non_eq => return non_eq,
            }
            i += 1;
        }

        Ordering::Equal
    }

    fn pre_tag_rank(tag: &str) -> PyResult<u8> {
        match tag {
            "a" => Ok(0),
            "b" => Ok(1),
            "rc" => Ok(2),
            _ => Err(PyValueError::new_err(format!(
                "invalid pre-release tag: {tag}"
            ))),
        }
    }

    fn compare_ascii_ci(left: &[u8], right: &[u8]) -> Ordering {
        let shared = left.len().min(right.len());
        for i in 0..shared {
            let l = left[i];
            let r = right[i];
            match l.to_ascii_lowercase().cmp(&r.to_ascii_lowercase()) {
                Ordering::Equal => {}
                non_eq => return non_eq,
            }
        }
        left.len().cmp(&right.len())
    }

    fn is_ascii_numeric(segment: &[u8]) -> bool {
        !segment.is_empty() && segment.iter().all(|b| b.is_ascii_digit())
    }

    fn compare_numeric_segments(left: &[u8], right: &[u8]) -> Ordering {
        let mut left_i = 0usize;
        while left_i < left.len() && left[left_i] == b'0' {
            left_i += 1;
        }

        let mut right_i = 0usize;
        while right_i < right.len() && right[right_i] == b'0' {
            right_i += 1;
        }

        let left_norm = if left_i == left.len() {
            &b"0"[..]
        } else {
            &left[left_i..]
        };
        let right_norm = if right_i == right.len() {
            &b"0"[..]
        } else {
            &right[right_i..]
        };

        match left_norm.len().cmp(&right_norm.len()) {
            Ordering::Equal => left_norm.cmp(right_norm),
            non_eq => non_eq,
        }
    }

    fn compare_local_segments(left: &[u8], right: &[u8]) -> Ordering {
        let left_numeric = is_ascii_numeric(left);
        let right_numeric = is_ascii_numeric(right);

        match (left_numeric, right_numeric) {
            (true, true) => compare_numeric_segments(left, right),
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (false, false) => compare_ascii_ci(left, right),
        }
    }

    fn next_local_segment(bytes: &[u8], mut i: usize) -> (&[u8], usize) {
        let start = i;
        while i < bytes.len() {
            let b = bytes[i];
            if b == b'.' || b == b'-' || b == b'_' {
                break;
            }
            i += 1;
        }

        let segment = &bytes[start..i];
        let next_i = if i < bytes.len() { i + 1 } else { i };
        (segment, next_i)
    }

    fn compare_local(left: Option<&str>, right: Option<&str>) -> Ordering {
        match (left, right) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            (Some(left), Some(right)) => {
                let left_bytes = left.as_bytes();
                let right_bytes = right.as_bytes();
                let mut left_i = 0usize;
                let mut right_i = 0usize;

                loop {
                    let left_done = left_i >= left_bytes.len();
                    let right_done = right_i >= right_bytes.len();
                    if left_done && right_done {
                        return Ordering::Equal;
                    }
                    if left_done {
                        return Ordering::Less;
                    }
                    if right_done {
                        return Ordering::Greater;
                    }

                    let (l, next_left_i) = next_local_segment(left_bytes, left_i);
                    let (r, next_right_i) = next_local_segment(right_bytes, right_i);
                    match compare_local_segments(l, r) {
                        Ordering::Equal => {}
                        non_eq => return non_eq,
                    }
                    left_i = next_left_i;
                    right_i = next_right_i;
                }
            }
        }
    }

    fn compare_version_tuples(left: &VersionTuple, right: &VersionTuple) -> PyResult<Ordering> {
        let left_epoch = left.0.unwrap_or(0);
        let right_epoch = right.0.unwrap_or(0);
        match left_epoch.cmp(&right_epoch) {
            Ordering::Equal => {}
            non_eq => return Ok(non_eq),
        }

        match compare_release(&left.1, &right.1) {
            Ordering::Equal => {}
            non_eq => return Ok(non_eq),
        }

        if let Some((left_tag, _)) = &left.2 {
            pre_tag_rank(left_tag)?;
        }
        if let Some((right_tag, _)) = &right.2 {
            pre_tag_rank(right_tag)?;
        }

        let left_pre_bucket = if left.2.is_none() && left.3.is_none() && left.4.is_some() {
            -1i8
        } else if left.2.is_some() {
            0
        } else {
            1
        };
        let right_pre_bucket = if right.2.is_none() && right.3.is_none() && right.4.is_some() {
            -1i8
        } else if right.2.is_some() {
            0
        } else {
            1
        };
        match left_pre_bucket.cmp(&right_pre_bucket) {
            Ordering::Equal => {}
            non_eq => return Ok(non_eq),
        }

        if let (Some((left_tag, left_num)), Some((right_tag, right_num))) = (&left.2, &right.2) {
            match pre_tag_rank(left_tag)?.cmp(&pre_tag_rank(right_tag)?) {
                Ordering::Equal => {}
                non_eq => return Ok(non_eq),
            }
            match left_num.cmp(right_num) {
                Ordering::Equal => {}
                non_eq => return Ok(non_eq),
            }
        }

        match (left.3, right.3) {
            (Some(l), Some(r)) => match l.cmp(&r) {
                Ordering::Equal => {}
                non_eq => return Ok(non_eq),
            },
            (None, Some(_)) => return Ok(Ordering::Less),
            (Some(_), None) => return Ok(Ordering::Greater),
            (None, None) => {}
        }

        match (left.4, right.4) {
            (Some(l), Some(r)) => match l.cmp(&r) {
                Ordering::Equal => {}
                non_eq => return Ok(non_eq),
            },
            (Some(_), None) => return Ok(Ordering::Less),
            (None, Some(_)) => return Ok(Ordering::Greater),
            (None, None) => {}
        }

        Ok(compare_local(left.5.as_deref(), right.5.as_deref()))
    }

    #[pyfunction]
    fn split_version(py: Python, version: &str) -> PyResult<Py<PyTuple>> {
        let version = version.trim();
        if version.is_empty() {
            return Err(PyValueError::new_err(
                "version segments must be valid (64-bit) integers",
            ));
        }

        let mut parts =
            Vec::with_capacity(version.as_bytes().iter().filter(|&&b| b == b'.').count() + 1);
        for segment in version.split('.') {
            let value = segment.parse::<u64>().map_err(|_| {
                PyValueError::new_err("version segments must be valid (64-bit) integers")
            })?;
            parts.push(value);
        }
        Ok(PyTuple::new(py, parts)?.into())
    }

    #[pyfunction]
    fn parse_version(py: Python, version: &str) -> PyResult<ParseVersionResult> {
        let version = version.trim();
        let bytes = version.as_bytes();
        let mut i = 0usize;

        if i < bytes.len() && (bytes[i] == b'v' || bytes[i] == b'V') {
            i = 1;
        }

        let mut epoch = None;
        if let Some((epoch_u64, next_i)) = parse_digits(bytes, i)
            && next_i < bytes.len()
            && bytes[next_i] == b'!'
        {
            let parsed_epoch = i64::try_from(epoch_u64).map_err(|_| {
                PyValueError::new_err("epoch is too large to fit into a 64-bit signed integer")
            })?;
            epoch = Some(parsed_epoch);
            i = next_i + 1;
        }

        let mut release: SmallVec<[u64; 4]> = SmallVec::new();
        let (first_release, mut next_i) = parse_digits(bytes, i)
            .ok_or_else(|| PyValueError::new_err("invalid version: expected release segment"))?;
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
        if i < bytes.len()
            && bytes[i] == b'-'
            && let Some((n, parsed_i)) = parse_digits(bytes, i + 1)
        {
            post = Some(n);
            i = parsed_i;
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

    #[pyfunction]
    fn cmp_version(left: VersionTuple, right: VersionTuple) -> PyResult<i8> {
        Ok(match compare_version_tuples(&left, &right)? {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        })
    }
}
