//! Chart data builders for oneshot mode: build bar, histogram, heatmap, and line/scatter data.

use crate::chart::data_builder::{self, ResolvedAxes};
use crate::chart::selector::{ChartRecommendation, ChartType};
use crate::cli::{AggFunction, SortOrder};
use crate::render::{BarChartData, ChartConfig, HistogramData};

use super::RenderOptions;

/// Build ChartConfig for Line/Scatter charts, including extra Y columns.
pub fn build_line_scatter_config(
    recommendation: &ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
    opts: &RenderOptions<'_>,
    area: ratatui::layout::Rect,
    chart_type: ChartType,
) -> ChartConfig {
    let mut config = build_chart_config(recommendation, headers, rows);
    if let Some(label) = opts.y_label_override {
        config.y_axis.label = label.to_string();
    }
    if !opts.extra_y_columns.is_empty() {
        apply_extra_y_columns(&mut config, recommendation, headers, rows, opts);
    }
    config.x_labels = config
        .x_labels
        .map(|labels| super::fit_labels_to_width(&labels, area.width.saturating_sub(12) as usize));
    let rendered = config.series.iter().map(|s| s.data.len()).sum::<usize>();
    let effective_rows = rows.len().min(data_builder::MAX_CHART_POINTS);
    super::warn_skipped_rows(effective_rows, rendered, recommendation, chart_type);
    config
}

/// Build base ChartConfig from recommendation.
fn build_chart_config(
    recommendation: &ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
) -> ChartConfig {
    let axes = ResolvedAxes::from_recommendation(
        &recommendation.x_column,
        recommendation.y_column.as_deref(),
        recommendation.color_column.as_deref(),
        headers,
    );
    let title = format!("{} vs {}", axes.y_label, axes.x_label);

    data_builder::build_chart_config(
        rows,
        axes.x_idx,
        axes.y_idx,
        axes.color_idx,
        axes.x_label,
        axes.y_label,
        Some(title),
    )
}

/// Sort bar chart data by value. No-op if sort_order is None or SortOrder::None.
pub fn sort_bar_data(data: &mut BarChartData, sort_order: Option<SortOrder>) {
    let reverse = match sort_order {
        Some(SortOrder::Desc) => true,
        Some(SortOrder::Asc) => false,
        _ => return,
    };
    let mut indices: Vec<usize> = (0..data.values.len()).collect();
    indices.sort_by(|a, b| {
        let cmp = data.values[*a]
            .partial_cmp(&data.values[*b])
            .unwrap_or(std::cmp::Ordering::Equal);
        if reverse { cmp.reverse() } else { cmp }
    });
    data.labels = indices.iter().map(|&i| data.labels[i].clone()).collect();
    data.values = indices.iter().map(|&i| data.values[i]).collect();
}

/// Truncate bar chart to first N categories. No-op if limit is None.
pub fn truncate_bar_data(data: &mut BarChartData, limit: Option<usize>) {
    if let Some(n) = limit {
        data.labels.truncate(n);
        data.values.truncate(n);
    }
}

/// Build BarChartData: aggregates values by category.
/// Returns (data, rows_used).
pub fn build_bar_data(
    recommendation: &ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
    agg: AggFunction,
) -> (BarChartData, usize) {
    let axes = ResolvedAxes::from_recommendation(
        &recommendation.x_column,
        recommendation.y_column.as_deref(),
        recommendation.color_column.as_deref(),
        headers,
    );
    let title = format!("{} by {}", axes.y_label, axes.x_label);

    data_builder::aggregate_bar(rows, axes.x_idx, axes.y_idx, Some(title), axes.y_label, agg)
}

/// Build HistogramData for Histogram charts.
pub fn build_histogram_data(
    recommendation: &ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
) -> HistogramData {
    let axes = ResolvedAxes::from_recommendation(
        &recommendation.x_column,
        recommendation.y_column.as_deref(),
        recommendation.color_column.as_deref(),
        headers,
    );

    // For histogram, prefer the quantitative column.
    let x_numeric_count = rows
        .iter()
        .take(5)
        .filter_map(|r| r.get(axes.x_idx))
        .filter(|v| v.parse::<f64>().is_ok())
        .count();

    let use_idx = if x_numeric_count > 0 {
        axes.x_idx
    } else {
        axes.y_idx
    };
    let label = headers.get(use_idx).cloned().unwrap_or_default();
    let title = format!("Distribution of {}", label);

    data_builder::build_histogram(rows, use_idx, Some(title), label)
}

/// Build heatmap data for two categorical columns.
pub fn build_heatmap(
    recommendation: &ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
) -> crate::render::HeatmapData {
    let axes = ResolvedAxes::from_recommendation(
        &recommendation.x_column,
        recommendation.y_column.as_deref(),
        recommendation.color_column.as_deref(),
        headers,
    );
    let title = format!("{} × {}", axes.x_label, axes.y_label);
    data_builder::build_heatmap_data(rows, axes.x_idx, axes.y_idx, Some(title))
}

/// Append extra Y columns as additional series and recalculate Y axis bounds.
fn apply_extra_y_columns(
    config: &mut ChartConfig,
    recommendation: &ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
    opts: &RenderOptions<'_>,
) {
    use crate::render::Axis;

    let axes = ResolvedAxes::from_recommendation(
        &recommendation.x_column,
        recommendation.y_column.as_deref(),
        recommendation.color_column.as_deref(),
        headers,
    );
    let raw_x: Vec<String> = rows
        .iter()
        .filter_map(|r| r.get(axes.x_idx).cloned())
        .collect();
    let x_is_non_numeric = data_builder::is_non_numeric(&raw_x);
    let y_specs: Vec<(usize, String)> = opts
        .extra_y_columns
        .iter()
        .filter_map(|(col, label)| {
            let idx = data_builder::column_index(headers, col)?;
            let name = label.as_deref().unwrap_or(col).to_string();
            Some((idx, name))
        })
        .collect();
    let extra = data_builder::build_multi_y_series(rows, axes.x_idx, &y_specs, x_is_non_numeric);
    config.series.extend(extra);
    let all_y: Vec<f64> = config
        .series
        .iter()
        .flat_map(|s| s.data.iter().map(|(_, y)| *y))
        .collect();
    config.y_axis = Axis::from_data(&config.y_axis.label, &all_y);
}
