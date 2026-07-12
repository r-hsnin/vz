//! Shared sparkline rendering utility.
//!
//! Used by both `--output spark` mode and the summary line decoration.

/// Unicode block characters for sparkline rendering (8 levels).
const BLOCKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Render a sparkline string from numeric values.
///
/// Returns an empty string for empty input.
/// For constant values (min == max), returns a flat mid-level bar.
pub fn sparkline_from_values(values: &[f64]) -> String {
    if values.is_empty() {
        return String::new();
    }
    let (min, max) = crate::util::min_max(values).unwrap_or((0.0, 0.0));
    if (max - min).abs() < f64::EPSILON {
        return "▄".repeat(values.len());
    }
    values
        .iter()
        .map(|&v| {
            let idx = ((v - min) / (max - min) * 7.0).round() as usize;
            BLOCKS[idx.min(7)]
        })
        .collect()
}

/// Sample values to at most `max_len` points (evenly spaced).
pub fn sample_values(values: &[f64], max_len: usize) -> Vec<f64> {
    if values.len() <= max_len {
        return values.to_vec();
    }
    let step = values.len() as f64 / max_len as f64;
    (0..max_len)
        .map(|i| values[(i as f64 * step) as usize])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparkline_empty() {
        assert_eq!(sparkline_from_values(&[]), "");
    }

    #[test]
    fn test_sparkline_single_value() {
        assert_eq!(sparkline_from_values(&[5.0]), "▄");
    }

    #[test]
    fn test_sparkline_constant_values() {
        assert_eq!(sparkline_from_values(&[3.0, 3.0, 3.0]), "▄▄▄");
    }

    #[test]
    fn test_sparkline_ascending() {
        let result = sparkline_from_values(&[0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]);
        assert_eq!(result, "▁▂▃▄▅▆▇█");
    }

    #[test]
    fn test_sparkline_two_values() {
        let result = sparkline_from_values(&[0.0, 10.0]);
        assert_eq!(result, "▁█");
    }

    #[test]
    fn test_sample_values_no_reduction() {
        let vals = vec![1.0, 2.0, 3.0];
        assert_eq!(sample_values(&vals, 8), vals);
    }

    #[test]
    fn test_sample_values_reduces() {
        let vals: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let sampled = sample_values(&vals, 8);
        assert_eq!(sampled.len(), 8);
        assert_eq!(sampled[0], 0.0);
    }
}
