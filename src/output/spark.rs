//! Spark output: sparkline rendering from chart data.

use std::collections::BTreeMap;

use crate::chart::data_builder;
use crate::chart::selector::{ChartRecommendation, ChartType};
use crate::cli::{AggFunction, SortOrder};
use crate::oneshot;
use crate::sparkline;

/// Parameters for spark output (avoids passing full Cli struct).
pub struct SparkParams {
    pub chart_type_override: Option<crate::cli::ChartTypeArg>,
    pub agg: AggFunction,
    pub sort: Option<SortOrder>,
    pub limit: Option<usize>,
    pub color_col: Option<String>,
}

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
    let Some(yi) = y_idx else {
        println!("▄");
        return;
    };

    let chart_type = oneshot::resolve_chart_type(recommendation, params.chart_type_override);

    // For bar charts, aggregate values by category then sparkline
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
        println!("{y_name}  {spark}");
        return;
    }

    // If color column specified, output one sparkline per group
    if let Some(ref color) = params.color_col
        && let Some(ci) = data_builder::column_index(headers, color)
    {
        let mut groups: BTreeMap<&str, Vec<f64>> = BTreeMap::new();
        for row in rows {
            let group = row.get(ci).map_or("", |v| v.as_str());
            let val = row.get(yi).and_then(|v| v.parse::<f64>().ok());
            if let Some(v) = val {
                groups.entry(group).or_default().push(v);
            }
        }
        for (name, values) in &groups {
            println!("{}  {}", name, make_sparkline(values));
        }
        return;
    }

    // Single sparkline from all Y values in row order
    let values: Vec<f64> = rows
        .iter()
        .filter_map(|r| r.get(yi)?.parse::<f64>().ok())
        .collect();
    let spark = make_sparkline(&values);
    println!("{y_name}  {spark}");
}

/// Generate a sparkline string from values.
fn make_sparkline(values: &[f64]) -> String {
    sparkline::sparkline_from_values(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::selector::ChartRecommendation;

    fn make_recommendation(x: &str, y: Option<&str>, color: Option<&str>) -> ChartRecommendation {
        ChartRecommendation {
            chart_type: ChartType::Line,
            x_column: x.to_string(),
            y_column: y.map(|s| s.to_string()),
            color_column: color.map(|s| s.to_string()),
        }
    }

    fn default_params() -> SparkParams {
        SparkParams {
            chart_type_override: None,
            agg: AggFunction::Sum,
            sort: None,
            limit: None,
            color_col: None,
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
        let rec = make_recommendation("x", None, None);
        let headers = vec!["x".to_string(), "y".to_string()];
        let rows = vec![vec!["a".to_string(), "1".to_string()]];
        // Just verify it doesn't panic
        print_spark(&rec, &headers, &rows, &default_params());
    }

    #[test]
    fn test_print_spark_basic_values() {
        let rec = make_recommendation("date", Some("value"), None);
        let headers = vec!["date".to_string(), "value".to_string()];
        let rows = vec![
            vec!["2024-01".to_string(), "10".to_string()],
            vec!["2024-02".to_string(), "20".to_string()],
            vec!["2024-03".to_string(), "30".to_string()],
        ];
        // Should not panic
        print_spark(&rec, &headers, &rows, &default_params());
    }
}
