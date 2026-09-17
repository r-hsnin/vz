//! Spark output: sparkline rendering from chart data.

use std::collections::BTreeMap;

use crate::chart::data_builder;
use crate::chart::selector::{ChartRecommendation, ChartType};
use crate::cli::{AggFunction, SortOrder};
use crate::oneshot;
use crate::render::format_number;
use crate::sparkline;
use crate::util;

/// Parameters for spark output (avoids passing full Cli struct).
pub struct SparkParams {
    pub chart_type_override: Option<crate::cli::ChartTypeArg>,
    pub agg: AggFunction,
    pub sort: Option<SortOrder>,
    pub limit: Option<usize>,
    pub color_col: Option<String>,
    pub bins: Option<usize>,
    pub extra_y_columns: Vec<String>,
}

/// Maximum raw points rendered into a single sparkline (sampled beyond this).
pub const SPARK_MAX_POINTS: usize = 200;

/// Print sparkline output: single-line, grouped, or aggregated for bar charts.
pub fn print_spark(
    recommendation: &ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
    params: &SparkParams,
) {
    let x_idx = data_builder::column_index(headers, &recommendation.x_column);
    let y_idx = recommendation
        .y_column
        .as_ref()
        .and_then(|y| data_builder::column_index(headers, y));
    let y_name = recommendation.y_column.as_deref().unwrap_or("value");

    let chart_type = oneshot::resolve_chart_type(recommendation, params.chart_type_override);

    // Histogram with no Y column: bin the X column values and sparkline the counts
    if y_idx.is_none() && chart_type == ChartType::Histogram {
        if let Some(xi) = x_idx {
            let values: Vec<f64> = rows
                .iter()
                .filter_map(|r| r.get(xi).and_then(|v| crate::util::parse_number(v)))
                .filter(|v| v.is_finite())
                .collect();
            if values.is_empty() {
                println!("{}", recommendation.x_column);
                return;
            }
            let bin_count = params.bins.unwrap_or(data_builder::DEFAULT_BINS);
            let bins = crate::render::compute_bins(&values, bin_count);
            let counts: Vec<f64> = bins.iter().map(|(_, _, c)| *c as f64).collect();
            let spark = make_sparkline(&counts);
            let range = util::min_max(&values)
                .map(|(min, max)| format!("({}–{})", format_number(min), format_number(max)))
                .unwrap_or_default();
            let x_name = &recommendation.x_column;
            println!("{x_name}  {spark}  {range} {} rows", values.len());
            return;
        }
        println!("▄");
        return;
    }

    let Some(yi) = y_idx else {
        println!("▄");
        return;
    };

    // For bar charts, aggregate values by category then sparkline.
    // Bar categories have no time order, so an endpoint trend (↑/↓) would be
    // a sorting artifact — show range only.
    if chart_type == ChartType::Bar
        && let Some(xi) = x_idx
    {
        let (mut bar_data, _) =
            data_builder::aggregate_bar(rows, xi, yi, None, String::new(), params.agg);
        oneshot::builders::sort_bar_data(&mut bar_data, params.sort);
        if let Some(n) = params.limit {
            bar_data.labels.truncate(n);
            bar_data.values.truncate(n);
        }
        let spark = make_sparkline(&bar_data.values);
        let suffix = range_suffix(&bar_data.values);
        println!("{y_name}  {spark}{suffix}");
        // Extra Y columns share the same X grouping: one line each.
        for extra in &params.extra_y_columns {
            if let Some(eyi) = data_builder::column_index(headers, extra) {
                let (mut extra_data, _) =
                    data_builder::aggregate_bar(rows, xi, eyi, None, String::new(), params.agg);
                oneshot::builders::sort_bar_data(&mut extra_data, params.sort);
                if let Some(n) = params.limit {
                    extra_data.labels.truncate(n);
                    extra_data.values.truncate(n);
                }
                let spark = make_sparkline(&extra_data.values);
                let suffix = range_suffix(&extra_data.values);
                println!("{extra}  {spark}{suffix}");
            }
        }
        return;
    }

    // If color column specified, output one sparkline per group
    if let Some(ref color) = params.color_col
        && let Some(ci) = data_builder::column_index(headers, color)
    {
        let mut groups: BTreeMap<&str, Vec<f64>> = BTreeMap::new();
        for row in rows {
            let group = row.get(ci).map_or("", |v| v.as_str());
            let val = row
                .get(yi)
                .and_then(|v| crate::util::parse_number(v))
                .filter(|v| v.is_finite());
            if let Some(v) = val {
                groups.entry(group).or_default().push(v);
            }
        }
        for (name, values) in &groups {
            let spark = make_sparkline(values);
            let suffix = stats_suffix(values);
            println!("{name}  {spark}{suffix}");
        }
        return;
    }

    // Single sparkline from all Y values in row order (non-finite skipped),
    // plus one line per extra Y column.
    let values: Vec<f64> = rows
        .iter()
        .filter_map(|r| r.get(yi).and_then(|v| crate::util::parse_number(v)))
        .filter(|v| v.is_finite())
        .collect();
    print_series_spark(y_name, &values, rows.len());
    for extra in &params.extra_y_columns {
        if let Some(eyi) = data_builder::column_index(headers, extra) {
            let extra_values: Vec<f64> = rows
                .iter()
                .filter_map(|r| r.get(eyi).and_then(|v| crate::util::parse_number(v)))
                .filter(|v| v.is_finite())
                .collect();
            print_series_spark(extra, &extra_values, rows.len());
        }
    }
}

/// Print one `name  spark  (range) trend` line, sampling long series and
/// reporting skipped non-finite values so log consumers see a stable 1-line contract.
fn print_series_spark(name: &str, values: &[f64], total_rows: usize) {
    let sampled: Vec<f64> = if values.len() > SPARK_MAX_POINTS {
        sparkline::sample_values(values, SPARK_MAX_POINTS)
    } else {
        values.to_vec()
    };
    let spark = make_sparkline(&sampled);
    let suffix = stats_suffix(values);
    let skipped = total_rows.saturating_sub(values.len());
    if skipped > 0 {
        println!("{name}  {spark}{suffix} ({skipped} skipped)");
    } else {
        println!("{name}  {spark}{suffix}");
    }
}

/// Range-only suffix (no trend): for bar aggregations where endpoints are
/// category order, not time.
fn range_suffix(values: &[f64]) -> String {
    util::min_max(values)
        .map(|(min, max)| format!("  ({}–{})", format_number(min), format_number(max)))
        .unwrap_or_default()
}

/// Generate a sparkline string from values.
fn make_sparkline(values: &[f64]) -> String {
    sparkline::sparkline_from_values(values)
}

/// Generate a stats suffix with range and trend annotation.
/// Format: `(min–max) ↑ +N%` or `(min–max) → stable`
fn stats_suffix(values: &[f64]) -> String {
    let range_part = util::min_max(values)
        .map(|(min, max)| format!("({}–{})", format_number(min), format_number(max)));
    let trend_part = trend_from_values(values);
    match (range_part, trend_part) {
        (Some(r), Some(t)) => format!("  {} {}", r, t),
        (Some(r), None) => format!("  {}", r),
        (None, Some(t)) => format!("  {}", t),
        (None, None) => String::new(),
    }
}

/// Compute trend annotation from a slice of values.
/// Returns arrow + percentage change from first to last value.
fn trend_from_values(values: &[f64]) -> Option<String> {
    if values.len() < 2 {
        return None;
    }
    let first = values[0];
    let last = *values.last()?;
    if first.abs() < f64::EPSILON {
        return None;
    }
    let pct = ((last - first) / first.abs()) * 100.0;
    if pct > 5.0 {
        Some(format!("↑ {:+.0}%", pct))
    } else if pct < -5.0 {
        Some(format!("↓ {:+.0}%", pct))
    } else {
        Some("→ stable".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::selector::ChartType;
    use crate::test_helpers::make_recommendation;

    fn default_params() -> SparkParams {
        SparkParams {
            chart_type_override: None,
            agg: AggFunction::Sum,
            sort: None,
            limit: None,
            color_col: None,
            bins: None,
            extra_y_columns: vec![],
        }
    }

    #[test]
    fn test_make_sparkline_basic() {
        let result = make_sparkline(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert!(!result.is_empty());
        // Sparkline should contain block chars
        assert!(result.chars().all(|c| "▁▂▃▄▅▆▇█".contains(c) || c == ' '));
    }

    #[test]
    fn test_make_sparkline_empty() {
        let result = make_sparkline(&[]);
        assert!(result.is_empty() || result.chars().all(|c| c == '▁'));
    }

    #[test]
    fn test_make_sparkline_constant() {
        let result = make_sparkline(&[5.0, 5.0, 5.0]);
        // All same value → all same block char
        let chars: Vec<char> = result.chars().collect();
        assert_eq!(chars.len(), 3);
        assert!(chars.windows(2).all(|w| w[0] == w[1]));
    }

    #[test]
    fn test_make_sparkline_ascending() {
        let result = make_sparkline(&[0.0, 25.0, 50.0, 75.0, 100.0]);
        let chars: Vec<char> = result.chars().collect();
        assert_eq!(chars.len(), 5);
        // Should be monotonically non-decreasing in block height
        let heights = "▁▂▃▄▅▆▇█";
        let indices: Vec<usize> = chars
            .iter()
            .map(|c| heights.find(*c).unwrap_or(0))
            .collect();
        for i in 1..indices.len() {
            assert!(indices[i] >= indices[i - 1], "Expected ascending sparkline");
        }
    }

    #[test]
    fn test_print_spark_no_y_column() {
        // When y_column is None, should print a single block char
        let rec = make_recommendation(ChartType::Line, "x", None, None);
        let headers = vec!["x".to_string(), "y".to_string()];
        let rows = vec![vec!["a".to_string(), "1".to_string()]];
        // Just verify it doesn't panic
        print_spark(&rec, &headers, &rows, &default_params());
    }

    #[test]
    fn test_print_spark_basic_values() {
        let rec = make_recommendation(ChartType::Line, "date", Some("value"), None);
        let headers = vec!["date".to_string(), "value".to_string()];
        let rows = vec![
            vec!["2024-01".to_string(), "10".to_string()],
            vec!["2024-02".to_string(), "20".to_string()],
            vec!["2024-03".to_string(), "30".to_string()],
        ];
        // Should not panic
        print_spark(&rec, &headers, &rows, &default_params());
    }

    #[test]
    fn test_stats_suffix_ascending() {
        let values = vec![100.0, 200.0, 300.0, 400.0, 500.0];
        let suffix = stats_suffix(&values);
        assert!(suffix.contains("100"));
        assert!(suffix.contains("500"));
        assert!(suffix.contains("↑"));
    }

    #[test]
    fn test_stats_suffix_descending() {
        let values = vec![1000.0, 800.0, 400.0, 200.0];
        let suffix = stats_suffix(&values);
        assert!(suffix.contains("200"));
        assert!(suffix.contains("1"));
        assert!(suffix.contains("↓"));
    }

    #[test]
    fn test_stats_suffix_stable() {
        let values = vec![100.0, 101.0, 99.0, 100.5];
        let suffix = stats_suffix(&values);
        assert!(suffix.contains("→ stable"));
    }

    #[test]
    fn test_stats_suffix_empty() {
        let suffix = stats_suffix(&[]);
        assert!(suffix.is_empty());
    }

    #[test]
    fn test_stats_suffix_single_value() {
        let suffix = stats_suffix(&[42.0]);
        // Should have range but no trend (need 2+ values)
        assert!(suffix.contains("42"));
        assert!(!suffix.contains("↑"));
        assert!(!suffix.contains("↓"));
    }

    #[test]
    fn test_trend_from_values_growth() {
        let trend = trend_from_values(&[100.0, 200.0]);
        assert_eq!(trend, Some("↑ +100%".to_string()));
    }

    #[test]
    fn test_trend_from_values_decline() {
        let trend = trend_from_values(&[100.0, 50.0]);
        assert_eq!(trend, Some("↓ -50%".to_string()));
    }

    #[test]
    fn test_trend_from_values_stable() {
        let trend = trend_from_values(&[100.0, 103.0]);
        assert_eq!(trend, Some("→ stable".to_string()));
    }

    #[test]
    fn test_trend_from_values_too_few() {
        assert_eq!(trend_from_values(&[100.0]), None);
        assert_eq!(trend_from_values(&[]), None);
    }

    #[test]
    fn test_trend_from_values_zero_start() {
        // Division by zero guard
        assert_eq!(trend_from_values(&[0.0, 100.0]), None);
    }

    #[test]
    fn test_trend_from_values_negative_start_improves() {
        // -100 -> -50 is an improvement, must not report ↓
        let trend = trend_from_values(&[-100.0, -50.0]).expect("needs trend");
        assert!(trend.contains('↑'), "Expected ↑, got: {}", trend);
    }
}
