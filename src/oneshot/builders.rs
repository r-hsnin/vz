//! Chart data builders for oneshot mode: build bar, histogram, heatmap, and line/scatter data.

use crate::chart::data_builder::{self, ResolvedAxes};
use crate::chart::selector::AggFunction;
use crate::chart::selector::{ChartRecommendation, ChartType};
use crate::render::{BarChartData, ChartConfig, HistogramData};

use super::RenderOptions;

/// Build ChartConfig for Line/Scatter charts, including extra Y columns.
pub(crate) fn build_line_scatter_config(
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
pub(crate) fn build_chart_config(
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

// Post-aggregation Bar adapters (`sort_bar_data`, `truncate_bar_data`) live in
// the canonical assembler `crate::chart::data_builder`; use them from there.

/// Build BarChartData: aggregates values by category.
/// Returns (data, rows_used).
pub(crate) fn build_bar_data(
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
#[cfg(test)]
pub(crate) fn build_histogram_data(
    recommendation: &ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
) -> HistogramData {
    build_histogram_data_with_bins(recommendation, headers, rows, None)
}

/// Build histogram data with an explicit bin count override.
pub(crate) fn build_histogram_data_with_bins(
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

    // For histogram, bin the quantitative column (canonical choice: X when
    // numeric, otherwise Y — shared with JSON/present/insights).
    let use_idx = data_builder::histogram_column(rows, axes.x_idx, axes.y_idx);
    let label = headers.get(use_idx).cloned().unwrap_or_default();
    let title = format!("Distribution of {}", label);

    data_builder::build_histogram(rows, use_idx, Some(title), label, bins)
}

/// Build heatmap data for two categorical columns.
pub(crate) fn build_heatmap(
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
    let axes = ResolvedAxes::from_recommendation(
        &recommendation.x_column,
        recommendation.y_column.as_deref(),
        recommendation.color_column.as_deref(),
        headers,
    );
    // Mirror the canonical base sampling exactly: for non-numeric X the
    // coordinate is the sampled row index, so unsampled extra series would
    // misalign with the base series and overrun its X span. The sampling
    // notice is the base config's (`maybe_sample`); stay silent here.
    let sampled = (rows.len() > data_builder::MAX_CHART_POINTS)
        .then(|| data_builder::sample_rows(rows, data_builder::MAX_CHART_POINTS));
    let effective_rows = sampled.as_deref().unwrap_or(rows);
    let raw_x: Vec<String> = effective_rows
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
    let extra =
        data_builder::build_multi_y_series(effective_rows, axes.x_idx, &y_specs, x_is_non_numeric);
    data_builder::append_series_refit_y(config, extra);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::selector::ChartRecommendation;
    use crate::chart::selector::SortOrder;

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
    fn test_build_bar_data_sort_contract_lives_in_canonical_assembler() {
        let (mut data, _) = build_bar_data(
            &bar_recommendation(),
            &sales_headers(),
            &sales_rows(),
            AggFunction::Sum,
        );
        data_builder::sort_bar_data(&mut data, Some(SortOrder::Desc));
        assert_eq!(data.labels[0], "Tokyo"); // 3000 > 500
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
    fn test_build_histogram_data_non_numeric_x_uses_y() {
        let headers = vec!["month".to_string(), "temperature".to_string()];
        let rows = vec![
            vec!["Jan".into(), "5".into()],
            vec!["Feb".into(), "7".into()],
        ];
        let rec = ChartRecommendation {
            chart_type: ChartType::Histogram,
            x_column: "month".to_string(),
            y_column: Some("temperature".to_string()),
            color_column: None,
        };
        let data = build_histogram_data(&rec, &headers, &rows);
        assert_eq!(data.values, vec![5.0, 7.0]);
        assert_eq!(data.x_label, "temperature");
    }

    #[test]
    fn test_extra_y_series_share_base_sampling() {
        use ratatui::layout::Rect;

        // >MAX_CHART_POINTS rows with a non-numeric X: the base series is
        // sampled (X = sampled row index), so the extra-Y series must be
        // built from the same sampled rows or it misaligns and overruns.
        let headers = vec![
            "date".to_string(),
            "revenue".to_string(),
            "profit".to_string(),
        ];
        let rows: Vec<Vec<String>> = (0..data_builder::MAX_CHART_POINTS + 1)
            .map(|i| {
                vec![
                    format!("2024-01-{:02}", i % 28 + 1),
                    i.to_string(),
                    (i * 2).to_string(),
                ]
            })
            .collect();
        let rec = ChartRecommendation {
            chart_type: ChartType::Line,
            x_column: "date".to_string(),
            y_column: Some("revenue".to_string()),
            color_column: None,
        };
        let opts = RenderOptions {
            chart_type_override: None,
            y_label_override: None,
            width: None,
            height: None,
            sort_order: None,
            extra_y_columns: vec![("profit".to_string(), None)],
            limit: None,
            agg: AggFunction::Sum,
            title: None,
            labels: false,
            theme: crate::theme::Theme::dark(),
            bins: None,
        };
        let config = build_line_scatter_config(
            &rec,
            &headers,
            &rows,
            &opts,
            Rect::new(0, 0, 80, 24),
            ChartType::Line,
        );

        assert_eq!(config.series.len(), 2);
        let (base, extra) = (&config.series[0].data, &config.series[1].data);
        assert_eq!(base.len(), data_builder::MAX_CHART_POINTS);
        assert_eq!(
            extra.len(),
            base.len(),
            "extra-Y must be built from the same sampled rows as the base series"
        );
        for (b, e) in base.iter().zip(extra) {
            assert_eq!(e.0, b.0, "extra-Y X must match the base X coordinate");
            assert!(
                (e.1 - b.1 * 2.0).abs() < f64::EPSILON,
                "extra-Y point must come from the same row as the base point"
            );
        }
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
