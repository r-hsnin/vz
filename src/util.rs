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
