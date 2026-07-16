use std::path::Path;

/// Extract the file stem from a path as a `&str`.
///
/// Returns the file stem (filename without extension) if it is valid UTF-8,
/// otherwise returns `"unknown"`.
pub fn path_label(p: &Path) -> &str {
    p.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown")
}

/// Compute the minimum and maximum of a slice of f64 values.
/// Returns `None` if the slice is empty.
pub fn min_max(values: &[f64]) -> Option<(f64, f64)> {
    if values.is_empty() {
        return None;
    }
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
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
}
