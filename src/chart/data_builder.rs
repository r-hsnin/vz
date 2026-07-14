//! Shared data construction logic for chart rendering.
//!
//! Used by oneshot, explore, and present modes to avoid duplication.

use crate::cli::AggFunction;
use crate::render::{Axis, BarChartData, ChartConfig, HistogramData, Series};

/// Maximum number of data points rendered in line/scatter charts.
/// Beyond this threshold, rows are systematically sampled.
pub const MAX_CHART_POINTS: usize = 5000;

/// Pick `count` evenly spaced items from a slice of strings.
/// Returns all items if the slice is empty or `count >= items.len()`.
pub fn pick_evenly(items: &[String], count: usize) -> Vec<String> {
    if items.is_empty() || count >= items.len() {
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
    let (groups, rows_used) = collect_groups(rows, x_idx, y_idx, agg);

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
            series_colors: vec![],
            axis_color: None,
        },
        rows_used,
    )
}

/// Collect rows into per-category groups for aggregation.
fn collect_groups(
    rows: &[Vec<String>],
    x_idx: usize,
    y_idx: usize,
    agg: AggFunction,
) -> (Vec<(String, Vec<f64>)>, usize) {
    let mut groups: Vec<(String, Vec<f64>)> = Vec::new();
    let mut rows_used = 0usize;

    for row in rows {
        let label = match row.get(x_idx) {
            Some(l) if !l.is_empty() => l.clone(),
            _ => continue,
        };

        let value = if agg == AggFunction::Count {
            1.0
        } else {
            match row.get(y_idx).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) => v,
                None => continue,
            }
        };

        rows_used += 1;
        if let Some(entry) = groups.iter_mut().find(|(l, _)| l == &label) {
            entry.1.push(value);
        } else {
            groups.push((label, vec![value]));
        }
    }

    (groups, rows_used)
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

    let x_labels = compute_x_labels(x_is_non_numeric, &raw_x_strings);
    let (x_axis, y_axis) = axes_from_series(&series, &x_label, &y_label);

    ChartConfig {
        title,
        x_axis,
        y_axis,
        series,
        x_labels,
        series_colors: vec![],
        axis_color: None,
        label_color: None,
    }
}

/// Compute categorical X labels from raw strings (evenly sampled).
fn compute_x_labels(x_is_non_numeric: bool, raw_x_strings: &[String]) -> Option<Vec<String>> {
    if x_is_non_numeric && !raw_x_strings.is_empty() {
        let unique = unique_ordered(raw_x_strings);
        // Show all labels when close to target count (avoids confusing elision)
        let target = 5;
        if unique.len() <= target + 2 {
            Some(unique)
        } else {
            Some(pick_evenly(&unique, target))
        }
    } else {
        None
    }
}

/// Derive X and Y axes by collecting all data points from series.
fn axes_from_series(series: &[Series], x_label: &str, y_label: &str) -> (Axis, Axis) {
    let all_x: Vec<f64> = series
        .iter()
        .flat_map(|s| s.data.iter().map(|(x, _)| *x))
        .collect();
    let all_y: Vec<f64> = series
        .iter()
        .flat_map(|s| s.data.iter().map(|(_, y)| *y))
        .collect();
    (
        Axis::from_data(x_label, &all_x),
        Axis::from_data(y_label, &all_y),
    )
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
    bin_count: Option<usize>,
) -> HistogramData {
    let values: Vec<f64> = rows
        .iter()
        .filter_map(|r| r.get(col_idx).and_then(|v| v.parse().ok()))
        .collect();

    HistogramData {
        title,
        values,
        bin_count: bin_count.unwrap_or(10),
        x_label,
        axis_color: None,
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
#[path = "data_builder_tests.rs"]
mod tests;
