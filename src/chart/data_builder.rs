//! Shared data construction logic for chart rendering.
//!
//! Used by oneshot, explore, and present modes to avoid duplication.

use crate::cli::AggFunction;
use crate::render::{Axis, BarChartData, ChartConfig, HistogramData, Series};

/// Maximum number of data points rendered in line/scatter charts.
/// Beyond this threshold, rows are systematically sampled.
pub const MAX_CHART_POINTS: usize = 5000;

/// Pick `count` evenly spaced items from a slice of strings.
/// Returns all items if the slice is smaller than or equal to `count + 2`.
pub fn pick_evenly(items: &[String], count: usize) -> Vec<String> {
    if items.is_empty() {
        return vec![];
    }
    if items.len() <= count + 2 {
        return items.to_vec();
    }
    let step = (items.len() - 1) as f64 / (count - 1) as f64;
    (0..count)
        .map(|i| {
            let idx = (step * i as f64).round() as usize;
            items[idx.min(items.len() - 1)].clone()
        })
        .collect()
}

/// Detect if X column values are non-numeric (temporal/categorical strings).
/// Samples up to 5 values to determine.
pub fn is_non_numeric(values: &[String]) -> bool {
    !values.is_empty() && values.iter().take(5).all(|s| s.parse::<f64>().is_err())
}

/// Compute unique X values in order of first appearance.
pub fn unique_ordered(values: &[String]) -> Vec<String> {
    let mut seen = Vec::new();
    for v in values {
        if !seen.contains(v) {
            seen.push(v.clone());
        }
    }
    seen
}

/// Aggregate values by category label (sum).
/// Returns (aggregated data, number of rows successfully used).
pub fn aggregate_bar(
    rows: &[Vec<String>],
    x_idx: usize,
    y_idx: usize,
    title: Option<String>,
    y_label: String,
    agg: AggFunction,
) -> (BarChartData, usize) {
    // Collect values per category
    let mut groups: Vec<(String, Vec<f64>)> = Vec::new();
    let mut rows_used = 0usize;

    for row in rows {
        let label = match row.get(x_idx) {
            Some(l) if !l.is_empty() => l.clone(),
            _ => continue,
        };

        if agg == AggFunction::Count {
            rows_used += 1;
            if let Some(entry) = groups.iter_mut().find(|(l, _)| l == &label) {
                entry.1.push(1.0);
            } else {
                groups.push((label, vec![1.0]));
            }
            continue;
        }

        let value = match row.get(y_idx).and_then(|v| v.parse::<f64>().ok()) {
            Some(v) => v,
            None => continue,
        };

        rows_used += 1;
        if let Some(entry) = groups.iter_mut().find(|(l, _)| l == &label) {
            entry.1.push(value);
        } else {
            groups.push((label, vec![value]));
        }
    }

    let labels: Vec<String> = groups.iter().map(|(l, _)| l.clone()).collect();
    let values: Vec<f64> = groups
        .iter()
        .map(|(_, vals)| apply_agg(vals, agg))
        .collect();

    (
        BarChartData {
            title,
            labels,
            values,
            y_label,
            show_labels: false,
        },
        rows_used,
    )
}

/// Apply the aggregation function to a slice of values.
fn apply_agg(values: &[f64], agg: AggFunction) -> f64 {
    match agg {
        AggFunction::Sum => values.iter().sum(),
        AggFunction::Mean => {
            if values.is_empty() {
                0.0
            } else {
                values.iter().sum::<f64>() / values.len() as f64
            }
        }
        AggFunction::Count => values.len() as f64,
        AggFunction::Max => values.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        AggFunction::Min => values.iter().cloned().fold(f64::INFINITY, f64::min),
    }
}

/// Build grouped series by a color column.
/// Returns a Vec of named Series, each containing (x, y) data points.
pub fn build_grouped_series(
    rows: &[Vec<String>],
    x_idx: usize,
    y_idx: usize,
    color_idx: usize,
    x_is_non_numeric: bool,
) -> Vec<Series> {
    let unique_x: Vec<String> = if x_is_non_numeric {
        let raw: Vec<String> = rows.iter().filter_map(|r| r.get(x_idx).cloned()).collect();
        unique_ordered(&raw)
    } else {
        Vec::new()
    };

    let mut groups: Vec<(String, Vec<(f64, f64)>)> = Vec::new();

    for (i, row) in rows.iter().enumerate() {
        let group_name = row.get(color_idx).cloned().unwrap_or_default();
        let x = if x_is_non_numeric {
            let x_val = row.get(x_idx).cloned().unwrap_or_default();
            unique_x.iter().position(|v| *v == x_val).unwrap_or(i) as f64
        } else {
            row.get(x_idx)
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(i as f64)
        };
        let y = match row.get(y_idx).and_then(|v| v.parse::<f64>().ok()) {
            Some(v) => v,
            None => continue,
        };

        if let Some(entry) = groups.iter_mut().find(|(name, _)| name == &group_name) {
            entry.1.push((x, y));
        } else {
            groups.push((group_name, vec![(x, y)]));
        }
    }

    groups
        .into_iter()
        .map(|(name, data)| Series { name, data })
        .collect()
}

/// Build a single series from row data (no color grouping).
/// For non-numeric X, uses row index as X coordinate.
pub fn build_single_series(
    rows: &[Vec<String>],
    x_idx: usize,
    y_idx: usize,
    x_is_non_numeric: bool,
    series_name: String,
) -> Series {
    let data: Vec<(f64, f64)> = rows
        .iter()
        .enumerate()
        .filter_map(|(i, row)| {
            let x = if x_is_non_numeric {
                i as f64
            } else {
                row.get(x_idx)
                    .and_then(|v| v.parse::<f64>().ok())
                    .unwrap_or(i as f64)
            };
            let y = row.get(y_idx).and_then(|v| v.parse::<f64>().ok())?;
            Some((x, y))
        })
        .collect();

    Series {
        name: series_name,
        data,
    }
}

/// Systematically sample rows to at most `max_count` entries.
/// Preserves first and last rows, evenly spacing intermediate samples.
/// Returns the original slice (as owned Vec) if already under the limit.
pub fn sample_rows(rows: &[Vec<String>], max_count: usize) -> Vec<Vec<String>> {
    if rows.len() <= max_count {
        return rows.to_vec();
    }
    if max_count == 0 {
        return vec![];
    }
    if max_count == 1 {
        return vec![rows[0].clone()];
    }
    let step = (rows.len() - 1) as f64 / (max_count - 1) as f64;
    (0..max_count)
        .map(|i| {
            let idx = (step * i as f64).round() as usize;
            rows[idx.min(rows.len() - 1)].clone()
        })
        .collect()
}

/// Build a full ChartConfig from rows, handling both single and multi-series.
pub fn build_chart_config(
    rows: &[Vec<String>],
    x_idx: usize,
    y_idx: usize,
    color_idx: Option<usize>,
    x_label: String,
    y_label: String,
    title: Option<String>,
) -> ChartConfig {
    let sampled = maybe_sample(rows);
    let effective_rows = sampled.as_ref().map_or(rows, |s| s.as_slice());

    let raw_x_strings: Vec<String> = effective_rows
        .iter()
        .filter_map(|r| r.get(x_idx).cloned())
        .collect();
    let x_is_non_numeric = is_non_numeric(&raw_x_strings);

    let series = if let Some(c_idx) = color_idx {
        build_grouped_series(effective_rows, x_idx, y_idx, c_idx, x_is_non_numeric)
    } else {
        vec![build_single_series(
            effective_rows,
            x_idx,
            y_idx,
            x_is_non_numeric,
            y_label.clone(),
        )]
    };

    let all_x: Vec<f64> = series
        .iter()
        .flat_map(|s| s.data.iter().map(|(x, _)| *x))
        .collect();
    let all_y: Vec<f64> = series
        .iter()
        .flat_map(|s| s.data.iter().map(|(_, y)| *y))
        .collect();

    let x_labels = if x_is_non_numeric && !raw_x_strings.is_empty() {
        let unique = unique_ordered(&raw_x_strings);
        Some(pick_evenly(&unique, 5))
    } else {
        None
    };

    ChartConfig {
        title,
        x_axis: Axis::from_data(&x_label, &all_x),
        y_axis: Axis::from_data(&y_label, &all_y),
        series,
        x_labels,
    }
}

/// Sample large datasets to keep rendering fast. Returns None if no sampling needed.
fn maybe_sample(rows: &[Vec<String>]) -> Option<Vec<Vec<String>>> {
    if rows.len() > MAX_CHART_POINTS {
        eprintln!(
            "info: sampled {}/{} rows for chart rendering",
            MAX_CHART_POINTS,
            rows.len()
        );
        Some(sample_rows(rows, MAX_CHART_POINTS))
    } else {
        None
    }
}

/// Build HistogramData from rows.
pub fn build_histogram(
    rows: &[Vec<String>],
    col_idx: usize,
    title: Option<String>,
    x_label: String,
) -> HistogramData {
    let values: Vec<f64> = rows
        .iter()
        .filter_map(|r| r.get(col_idx).and_then(|v| v.parse().ok()))
        .collect();

    HistogramData {
        title,
        values,
        bin_count: 10,
        x_label,
    }
}

/// Find the index of a column name in headers.
pub fn column_index(headers: &[String], name: &str) -> Option<usize> {
    headers.iter().position(|h| h == name)
}

/// Resolved column indices and labels for chart rendering.
/// Shared across oneshot, explore, and present modes.
pub struct ResolvedAxes {
    pub x_idx: usize,
    pub y_idx: usize,
    pub color_idx: Option<usize>,
    pub x_label: String,
    pub y_label: String,
}

impl ResolvedAxes {
    /// Resolve axes from explicit column names (used by present mode / chart blocks).
    pub fn from_explicit(
        x_col: Option<&str>,
        y_col: Option<&str>,
        color_col: Option<&str>,
        headers: &[String],
    ) -> Self {
        let x_idx = x_col
            .and_then(|name| column_index(headers, name))
            .unwrap_or(0);
        let y_idx = y_col
            .and_then(|name| column_index(headers, name))
            .unwrap_or(1.min(headers.len().saturating_sub(1)));
        let color_idx = color_col.and_then(|name| column_index(headers, name));
        let x_label = headers.get(x_idx).cloned().unwrap_or_default();
        let y_label = headers.get(y_idx).cloned().unwrap_or_default();

        Self {
            x_idx,
            y_idx,
            color_idx,
            x_label,
            y_label,
        }
    }

    /// Resolve axes from a ChartRecommendation (used by oneshot/explore).
    pub fn from_recommendation(
        x_column: &str,
        y_column: Option<&str>,
        color_column: Option<&str>,
        headers: &[String],
    ) -> Self {
        Self::from_explicit(Some(x_column), y_column, color_column, headers)
    }
}

/// Build multiple series from multiple Y columns, sharing the same X axis.
/// Each (y_idx, label) pair produces one Series.
pub fn build_multi_y_series(
    rows: &[Vec<String>],
    x_idx: usize,
    y_specs: &[(usize, String)],
    x_is_non_numeric: bool,
) -> Vec<Series> {
    y_specs
        .iter()
        .map(|(y_idx, label)| {
            build_single_series(rows, x_idx, *y_idx, x_is_non_numeric, label.clone())
        })
        .collect()
}

/// Build a heatmap count matrix from two categorical columns.
/// Rows are unique values from `row_idx`, columns from `col_idx`.
pub fn build_heatmap_data(
    rows: &[Vec<String>],
    row_idx: usize,
    col_idx: usize,
    title: Option<String>,
) -> crate::render::HeatmapData {
    let row_labels = unique_ordered(
        &rows
            .iter()
            .filter_map(|r| r.get(row_idx).cloned())
            .collect::<Vec<_>>(),
    );
    let col_labels = unique_ordered(
        &rows
            .iter()
            .filter_map(|r| r.get(col_idx).cloned())
            .collect::<Vec<_>>(),
    );

    let row_map: std::collections::HashMap<&str, usize> = row_labels
        .iter()
        .enumerate()
        .map(|(i, l)| (l.as_str(), i))
        .collect();
    let col_map: std::collections::HashMap<&str, usize> = col_labels
        .iter()
        .enumerate()
        .map(|(i, l)| (l.as_str(), i))
        .collect();

    let mut counts = vec![vec![0usize; col_labels.len()]; row_labels.len()];
    for row in rows {
        let r_val = row.get(row_idx).map(|s| s.as_str()).unwrap_or("");
        let c_val = row.get(col_idx).map(|s| s.as_str()).unwrap_or("");
        if let (Some(&ri), Some(&ci)) = (row_map.get(r_val), col_map.get(c_val)) {
            counts[ri][ci] += 1;
        }
    }

    let max_count = counts.iter().flatten().copied().max().unwrap_or(0);

    crate::render::HeatmapData {
        title,
        row_labels,
        col_labels,
        counts,
        max_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pick_evenly_small() {
        let items: Vec<String> = vec!["a", "b", "c"].into_iter().map(String::from).collect();
        assert_eq!(pick_evenly(&items, 5), items);
    }

    #[test]
    fn test_pick_evenly_large() {
        let items: Vec<String> = (0..20).map(|i| format!("item_{}", i)).collect();
        let result = pick_evenly(&items, 5);
        assert_eq!(result.len(), 5);
        assert_eq!(result[0], "item_0");
        assert_eq!(result[4], "item_19");
    }

    #[test]
    fn test_pick_evenly_empty() {
        let items: Vec<String> = vec![];
        assert_eq!(pick_evenly(&items, 5), Vec::<String>::new());
    }

    #[test]
    fn test_is_non_numeric() {
        assert!(is_non_numeric(&["2024-01".into(), "2024-02".into()]));
        assert!(!is_non_numeric(&["1.0".into(), "2.5".into(), "3".into()]));
        assert!(!is_non_numeric(&[]));
    }

    #[test]
    fn test_unique_ordered() {
        let vals: Vec<String> = vec!["a", "b", "a", "c", "b"]
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(unique_ordered(&vals), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_aggregate_bar() {
        let rows = vec![
            vec!["Tokyo".into(), "1000".into()],
            vec!["Osaka".into(), "500".into()],
            vec!["Tokyo".into(), "2000".into()],
        ];
        let (data, used) = aggregate_bar(
            &rows,
            0,
            1,
            Some("Test".into()),
            "revenue".into(),
            AggFunction::Sum,
        );
        assert_eq!(data.labels, vec!["Tokyo", "Osaka"]);
        assert_eq!(data.values, vec![3000.0, 500.0]);
        assert_eq!(used, 3);
    }

    #[test]
    fn test_aggregate_bar_with_non_parseable() {
        let rows = vec![
            vec!["Tokyo".into(), "1000".into()],
            vec!["Osaka".into(), "bad".into()],
            vec!["Tokyo".into(), "500".into()],
        ];
        let (data, used) = aggregate_bar(&rows, 0, 1, None, "y".into(), AggFunction::Sum);
        assert_eq!(data.labels, vec!["Tokyo"]);
        assert_eq!(data.values, vec![1500.0]);
        assert_eq!(used, 2);
    }

    #[test]
    fn test_aggregate_bar_mean() {
        let rows = vec![
            vec!["Tokyo".into(), "1000".into()],
            vec!["Osaka".into(), "500".into()],
            vec!["Tokyo".into(), "3000".into()],
        ];
        let (data, used) = aggregate_bar(&rows, 0, 1, None, "y".into(), AggFunction::Mean);
        assert_eq!(data.labels, vec!["Tokyo", "Osaka"]);
        assert_eq!(data.values, vec![2000.0, 500.0]); // mean(1000,3000)=2000, mean(500)=500
        assert_eq!(used, 3);
    }

    #[test]
    fn test_aggregate_bar_count() {
        let rows = vec![
            vec!["Tokyo".into(), "1000".into()],
            vec!["Osaka".into(), "500".into()],
            vec!["Tokyo".into(), "3000".into()],
            vec!["Tokyo".into(), "bad".into()], // count still counts this
        ];
        let (data, used) = aggregate_bar(&rows, 0, 1, None, "y".into(), AggFunction::Count);
        assert_eq!(data.labels, vec!["Tokyo", "Osaka"]);
        assert_eq!(data.values, vec![3.0, 1.0]); // count ignores Y parsability
        assert_eq!(used, 4);
    }

    #[test]
    fn test_aggregate_bar_max_min() {
        let rows = vec![
            vec!["A".into(), "10".into()],
            vec!["A".into(), "30".into()],
            vec!["A".into(), "20".into()],
            vec!["B".into(), "5".into()],
        ];
        let (data_max, _) = aggregate_bar(&rows, 0, 1, None, "y".into(), AggFunction::Max);
        assert_eq!(data_max.values, vec![30.0, 5.0]);

        let (data_min, _) = aggregate_bar(&rows, 0, 1, None, "y".into(), AggFunction::Min);
        assert_eq!(data_min.values, vec![10.0, 5.0]);
    }

    #[test]
    fn test_build_single_series() {
        let rows = vec![
            vec!["1.0".into(), "10.0".into()],
            vec!["2.0".into(), "20.0".into()],
            vec!["3.0".into(), "30.0".into()],
        ];
        let series = build_single_series(&rows, 0, 1, false, "Y".into());
        assert_eq!(series.data.len(), 3);
        assert_eq!(series.data[0], (1.0, 10.0));
        assert_eq!(series.data[2], (3.0, 30.0));
    }

    #[test]
    fn test_build_grouped_series() {
        let rows = vec![
            vec!["2024-01".into(), "100".into(), "A".into()],
            vec!["2024-02".into(), "200".into(), "B".into()],
            vec!["2024-01".into(), "150".into(), "B".into()],
        ];
        let series = build_grouped_series(&rows, 0, 1, 2, true);
        assert_eq!(series.len(), 2);
        assert_eq!(series[0].name, "A");
        assert_eq!(series[1].name, "B");
        assert_eq!(series[0].data.len(), 1);
        assert_eq!(series[1].data.len(), 2);
    }

    #[test]
    fn test_build_chart_config_single_series() {
        let rows = vec![
            vec!["2024-01".into(), "100".into()],
            vec!["2024-02".into(), "200".into()],
        ];
        let config = build_chart_config(&rows, 0, 1, None, "date".into(), "value".into(), None);
        assert_eq!(config.series.len(), 1);
        assert_eq!(config.series[0].data.len(), 2);
        assert!(config.x_labels.is_some()); // non-numeric X
    }

    #[test]
    fn test_build_chart_config_multi_series() {
        let rows = vec![
            vec!["2024-01".into(), "100".into(), "A".into()],
            vec!["2024-01".into(), "200".into(), "B".into()],
            vec!["2024-02".into(), "150".into(), "A".into()],
        ];
        let config = build_chart_config(&rows, 0, 1, Some(2), "date".into(), "value".into(), None);
        assert_eq!(config.series.len(), 2);
    }

    #[test]
    fn test_column_index() {
        let headers: Vec<String> = vec!["a".into(), "b".into(), "c".into()];
        assert_eq!(column_index(&headers, "b"), Some(1));
        assert_eq!(column_index(&headers, "z"), None);
    }

    #[test]
    fn test_sample_rows_under_threshold() {
        // Under threshold: no sampling
        let rows: Vec<Vec<String>> = (0..100)
            .map(|i| vec![format!("{}", i), format!("{}", i * 10)])
            .collect();
        let sampled = sample_rows(&rows, 5000);
        assert_eq!(sampled.len(), 100);
    }

    #[test]
    fn test_sample_rows_over_threshold() {
        // Over threshold: sampled down
        let rows: Vec<Vec<String>> = (0..10000)
            .map(|i| vec![format!("{}", i), format!("{}", i * 10)])
            .collect();
        let sampled = sample_rows(&rows, 5000);
        assert_eq!(sampled.len(), 5000);
        // First and last rows should be preserved (systematic sampling)
        assert_eq!(sampled[0], rows[0]);
        assert_eq!(sampled[4999], rows[9999]);
    }

    #[test]
    fn test_sample_rows_empty() {
        let rows: Vec<Vec<String>> = vec![];
        let sampled = sample_rows(&rows, 5000);
        assert!(sampled.is_empty());
    }

    #[test]
    fn test_build_chart_config_samples_large_data() {
        // 10k rows should be sampled for chart rendering
        let rows: Vec<Vec<String>> = (0..10000)
            .map(|i| vec![format!("{}", i), format!("{}", i * 2)])
            .collect();
        let config = build_chart_config(&rows, 0, 1, None, "x".into(), "y".into(), None);
        // Should have at most MAX_CHART_POINTS data points
        let total_points: usize = config.series.iter().map(|s| s.data.len()).sum();
        assert!(total_points <= MAX_CHART_POINTS);
    }

    #[test]
    fn test_build_heatmap_data_basic() {
        let rows = vec![
            vec!["A".to_string(), "X".to_string()],
            vec!["A".to_string(), "Y".to_string()],
            vec!["B".to_string(), "X".to_string()],
            vec!["B".to_string(), "X".to_string()],
        ];
        let data = build_heatmap_data(&rows, 0, 1, Some("Test".to_string()));
        assert_eq!(data.row_labels, vec!["A", "B"]);
        assert_eq!(data.col_labels, vec!["X", "Y"]);
        assert_eq!(data.counts, vec![vec![1, 1], vec![2, 0]]);
        assert_eq!(data.max_count, 2);
        assert_eq!(data.title, Some("Test".to_string()));
    }

    #[test]
    fn test_build_heatmap_data_empty() {
        let rows: Vec<Vec<String>> = vec![];
        let data = build_heatmap_data(&rows, 0, 1, None);
        assert!(data.row_labels.is_empty());
        assert!(data.col_labels.is_empty());
        assert!(data.counts.is_empty());
        assert_eq!(data.max_count, 0);
    }

    #[test]
    fn test_build_heatmap_data_single_cell() {
        let rows = vec![
            vec!["A".to_string(), "X".to_string()],
            vec!["A".to_string(), "X".to_string()],
            vec!["A".to_string(), "X".to_string()],
        ];
        let data = build_heatmap_data(&rows, 0, 1, None);
        assert_eq!(data.row_labels, vec!["A"]);
        assert_eq!(data.col_labels, vec!["X"]);
        assert_eq!(data.counts, vec![vec![3]]);
        assert_eq!(data.max_count, 3);
    }
}

#[test]
fn test_resolved_axes_from_explicit() {
    let headers = vec!["city".into(), "revenue".into(), "region".into()];
    let axes = ResolvedAxes::from_explicit(Some("city"), Some("revenue"), Some("region"), &headers);
    assert_eq!(axes.x_idx, 0);
    assert_eq!(axes.y_idx, 1);
    assert_eq!(axes.color_idx, Some(2));
    assert_eq!(axes.x_label, "city");
    assert_eq!(axes.y_label, "revenue");
}

#[test]
fn test_resolved_axes_from_explicit_defaults() {
    let headers = vec!["date".into(), "value".into()];
    let axes = ResolvedAxes::from_explicit(None, None, None, &headers);
    assert_eq!(axes.x_idx, 0);
    assert_eq!(axes.y_idx, 1);
    assert_eq!(axes.color_idx, None);
    assert_eq!(axes.x_label, "date");
    assert_eq!(axes.y_label, "value");
}

#[test]
fn test_resolved_axes_from_recommendation() {
    let headers = vec!["month".into(), "sales".into(), "city".into()];
    let axes = ResolvedAxes::from_recommendation("month", Some("sales"), Some("city"), &headers);
    assert_eq!(axes.x_idx, 0);
    assert_eq!(axes.y_idx, 1);
    assert_eq!(axes.color_idx, Some(2));
}

#[test]
fn test_resolved_axes_single_column() {
    let headers = vec!["values".into()];
    let axes = ResolvedAxes::from_explicit(None, None, None, &headers);
    assert_eq!(axes.x_idx, 0);
    assert_eq!(axes.y_idx, 0); // min(1, len-1) = min(1, 0) = 0
}
