use std::path::Path;

/// Extract the file stem from a path as a `&str`.
///
/// Returns the file stem (filename without extension) if it is valid UTF-8,
/// otherwise returns `"unknown"`.
pub fn path_label(p: &Path) -> &str {
    p.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown")
}

/// Compute the minimum and maximum of a slice of f64 values.
/// Non-finite values (NaN/±inf) are ignored; returns `None` when the slice
/// is empty or contains no finite value.
pub fn min_max(values: &[f64]) -> Option<(f64, f64)> {
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for &v in values {
        if !v.is_finite() {
            continue;
        }
        if v < min {
            min = v;
        }
        if v > max {
            max = v;
        }
    }
    if min == f64::INFINITY {
        return None;
    }
    Some((min, max))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_path_label_with_extension() {
        assert_eq!(path_label(Path::new("sales.csv")), "sales");
    }

    #[test]
    fn test_path_label_without_extension() {
        assert_eq!(path_label(Path::new("data")), "data");
    }

    #[test]
    fn test_path_label_multiple_dots() {
        assert_eq!(path_label(Path::new("data.backup.csv")), "data.backup");
    }

    #[test]
    fn test_path_label_full_path() {
        assert_eq!(path_label(Path::new("/tmp/reports/q1.csv")), "q1");
    }

    #[test]
    fn test_path_label_empty_path() {
        assert_eq!(path_label(Path::new("")), "unknown");
    }

    #[test]
    fn test_min_max_normal() {
        assert_eq!(min_max(&[3.0, 1.0, 4.0, 1.5, 9.0]), Some((1.0, 9.0)));
    }

    #[test]
    fn test_min_max_single() {
        assert_eq!(min_max(&[42.0]), Some((42.0, 42.0)));
    }

    #[test]
    fn test_min_max_empty() {
        assert_eq!(min_max(&[]), None);
    }

    #[test]
    fn test_min_max_negative() {
        assert_eq!(min_max(&[-5.0, -1.0, -10.0]), Some((-10.0, -1.0)));
    }

    #[test]
    fn test_min_max_ignores_non_finite() {
        assert_eq!(min_max(&[1.0, f64::NAN, 3.0]), Some((1.0, 3.0)));
        assert_eq!(
            min_max(&[f64::INFINITY, 2.0, f64::NEG_INFINITY]),
            Some((2.0, 2.0))
        );
    }

    #[test]
    fn test_min_max_all_non_finite_returns_none() {
        assert_eq!(min_max(&[f64::NAN, f64::INFINITY]), None);
    }
}
