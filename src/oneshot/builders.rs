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
    // Apply theme colors
    config.apply_theme(&opts.theme);
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
    build_histogram_data_with_bins(recommendation, headers, rows, None)
}

/// Build histogram data with an explicit bin count override.
pub fn build_histogram_data_with_bins(
    recommendation: &ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
    bins: Option<usize>,
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

    data_builder::build_histogram(rows, use_idx, Some(title), label, bins)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::selector::ChartRecommendation;

    fn sales_headers() -> Vec<String> {
        vec![
            "city".to_string(),
            "revenue".to_string(),
            "profit".to_string(),
        ]
    }

    fn sales_rows() -> Vec<Vec<String>> {
        vec![
            vec!["Tokyo".into(), "1000".into(), "200".into()],
            vec!["Osaka".into(), "500".into(), "100".into()],
            vec!["Tokyo".into(), "2000".into(), "400".into()],
        ]
    }

    fn bar_recommendation() -> ChartRecommendation {
        ChartRecommendation {
            chart_type: ChartType::Bar,
            x_column: "city".to_string(),
            y_column: Some("revenue".to_string()),
            color_column: None,
        }
    }

    #[test]
    fn test_build_bar_data_aggregates() {
        let (data, rows_used) = build_bar_data(
            &bar_recommendation(),
            &sales_headers(),
            &sales_rows(),
            AggFunction::Sum,
        );
        assert_eq!(rows_used, 3);
        assert!(data.labels.contains(&"Tokyo".to_string()));
        assert!(data.labels.contains(&"Osaka".to_string()));
        // Tokyo sum = 3000, Osaka sum = 500
        let tokyo_idx = data.labels.iter().position(|l| l == "Tokyo").unwrap();
        assert!((data.values[tokyo_idx] - 3000.0).abs() < 0.01);
    }

    #[test]
    fn test_build_bar_data_mean() {
        let (data, _) = build_bar_data(
            &bar_recommendation(),
            &sales_headers(),
            &sales_rows(),
            AggFunction::Mean,
        );
        let tokyo_idx = data.labels.iter().position(|l| l == "Tokyo").unwrap();
        assert!((data.values[tokyo_idx] - 1500.0).abs() < 0.01); // (1000+2000)/2
    }

    #[test]
    fn test_sort_bar_data_desc() {
        let (mut data, _) = build_bar_data(
            &bar_recommendation(),
            &sales_headers(),
            &sales_rows(),
            AggFunction::Sum,
        );
        sort_bar_data(&mut data, Some(SortOrder::Desc));
        assert_eq!(data.labels[0], "Tokyo"); // 3000 > 500
    }

    #[test]
    fn test_sort_bar_data_asc() {
        let (mut data, _) = build_bar_data(
            &bar_recommendation(),
            &sales_headers(),
            &sales_rows(),
            AggFunction::Sum,
        );
        sort_bar_data(&mut data, Some(SortOrder::Asc));
        assert_eq!(data.labels[0], "Osaka"); // 500 < 3000
    }

    #[test]
    fn test_truncate_bar_data_limit() {
        let (mut data, _) = build_bar_data(
            &bar_recommendation(),
            &sales_headers(),
            &sales_rows(),
            AggFunction::Sum,
        );
        truncate_bar_data(&mut data, Some(1));
        assert_eq!(data.labels.len(), 1);
        assert_eq!(data.values.len(), 1);
    }

    #[test]
    fn test_sort_bar_data_none_preserves_order() {
        let mut data = BarChartData {
            labels: vec!["A".into(), "B".into(), "C".into()],
            values: vec![10.0, 30.0, 20.0],
            y_label: String::new(),
            title: None,
            show_labels: false,
            series_colors: vec![],
            axis_color: None,
        };
        sort_bar_data(&mut data, None);
        assert_eq!(data.labels, vec!["A", "B", "C"]);
    }

    #[test]
    fn test_sort_bar_data_with_nan() {
        let mut data = BarChartData {
            labels: vec!["A".into(), "B".into(), "C".into()],
            values: vec![f64::NAN, 30.0, 20.0],
            y_label: String::new(),
            title: None,
            show_labels: false,
            series_colors: vec![],
            axis_color: None,
        };
        sort_bar_data(&mut data, Some(SortOrder::Desc));
        let non_nan: Vec<(&str, f64)> = data
            .labels
            .iter()
            .zip(data.values.iter())
            .filter(|(_, v)| !v.is_nan())
            .map(|(l, v)| (l.as_str(), *v))
            .collect();
        assert_eq!(non_nan, vec![("B", 30.0), ("C", 20.0)]);
    }

    #[test]
    fn test_truncate_bar_data_none_noop() {
        let mut data = BarChartData {
            labels: vec!["A".into(), "B".into(), "C".into()],
            values: vec![100.0, 50.0, 25.0],
            y_label: "val".into(),
            title: None,
            show_labels: false,
            series_colors: vec![],
            axis_color: None,
        };
        truncate_bar_data(&mut data, None);
        assert_eq!(data.labels.len(), 3);
    }

    #[test]
    fn test_truncate_bar_data_larger_than_data() {
        let mut data = BarChartData {
            labels: vec!["A".into(), "B".into()],
            values: vec![100.0, 50.0],
            y_label: "val".into(),
            title: None,
            show_labels: false,
            series_colors: vec![],
            axis_color: None,
        };
        truncate_bar_data(&mut data, Some(10));
        assert_eq!(data.labels.len(), 2);
    }

    #[test]
    fn test_build_histogram_data_numeric_column() {
        let rec = ChartRecommendation {
            chart_type: ChartType::Histogram,
            x_column: "revenue".to_string(),
            y_column: None,
            color_column: None,
        };
        let data = build_histogram_data(&rec, &sales_headers(), &sales_rows());
        assert!(!data.values.is_empty());
        assert!(data.title.unwrap_or_default().contains("revenue"));
    }

    #[test]
    fn test_build_heatmap_two_categoricals() {
        let headers = vec!["city".to_string(), "product".to_string()];
        let rows = vec![
            vec!["Tokyo".into(), "A".into()],
            vec!["Tokyo".into(), "B".into()],
            vec!["Osaka".into(), "A".into()],
        ];
        let rec = ChartRecommendation {
            chart_type: ChartType::Heatmap,
            x_column: "city".to_string(),
            y_column: Some("product".to_string()),
            color_column: None,
        };
        let data = build_heatmap(&rec, &headers, &rows);
        assert!(!data.row_labels.is_empty());
        assert!(!data.col_labels.is_empty());
    }
}
