//! Text formatting for column statistics (used by --info).

use crate::infer::types::DataType;
use crate::loader::LoadedData;
use crate::output::{self, ColumnStats};

/// Compute and format column statistics as a human-readable string.
pub fn compute_column_stats_text(
    col_idx: usize,
    data_type: &DataType,
    data: &LoadedData,
) -> String {
    match output::compute_column_stats(col_idx, data_type, data) {
        ColumnStats::Quantitative { min, max, mean } => {
            format!(
                "Min={}  Max={}  Mean={}",
                format_stat(min),
                format_stat(max),
                format_stat(mean)
            )
        }
        ColumnStats::Categorical { unique, .. } => format!("{} unique", unique),
        ColumnStats::Temporal { min, max } => {
            if min == max {
                min
            } else {
                format!("{}..{}", min, max)
            }
        }
        ColumnStats::Empty {} => String::new(),
    }
}

/// Format a numeric stat value concisely.
pub fn format_stat(val: f64) -> String {
    if val == val.trunc() && val.abs() < 1_000_000.0 {
        format!("{:.0}", val)
    } else if val.abs() >= 1_000_000.0 {
        format!("{:.2e}", val)
    } else {
        format!("{:.2}", val)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_stat_integer() {
        assert_eq!(format_stat(42.0), "42");
        assert_eq!(format_stat(0.0), "0");
        assert_eq!(format_stat(-5.0), "-5");
    }

    #[test]
    fn test_format_stat_decimal() {
        assert_eq!(format_stat(3.75), "3.75");
        assert_eq!(format_stat(0.5), "0.50");
    }

    #[test]
    fn test_format_stat_large() {
        let result = format_stat(1_500_000.0);
        assert!(
            result.contains("e"),
            "Expected scientific notation: {}",
            result
        );
    }

    #[test]
    fn test_compute_column_stats_text_quantitative() {
        let data = LoadedData {
            headers: vec!["n".into()],
            rows: vec![vec!["10".into()], vec!["20".into()], vec!["30".into()]],
        };
        let result = compute_column_stats_text(0, &DataType::Quantitative, &data);
        assert!(result.contains("Min=10"));
        assert!(result.contains("Max=30"));
        assert!(result.contains("Mean=20"));
    }

    #[test]
    fn test_compute_column_stats_text_categorical() {
        let data = LoadedData {
            headers: vec!["city".into()],
            rows: vec![
                vec!["Tokyo".into()],
                vec!["Osaka".into()],
                vec!["Tokyo".into()],
            ],
        };
        let result = compute_column_stats_text(0, &DataType::Categorical, &data);
        assert_eq!(result, "2 unique");
    }

    #[test]
    fn test_compute_column_stats_text_temporal() {
        let data = LoadedData {
            headers: vec!["date".into()],
            rows: vec![vec!["2024-01".into()], vec!["2024-06".into()]],
        };
        let result = compute_column_stats_text(0, &DataType::Temporal, &data);
        assert_eq!(result, "2024-01..2024-06");
    }

    #[test]
    fn test_compute_column_stats_text_empty() {
        let data = LoadedData {
            headers: vec!["x".into()],
            rows: vec![vec!["".into()]],
        };
        let result = compute_column_stats_text(0, &DataType::Quantitative, &data);
        assert_eq!(result, "");
    }
}
