pub mod ansi;
mod summary;

use std::io;

use ratatui::{buffer::Buffer, layout::Rect};

use crate::chart::data_builder;
use crate::chart::selector::{ChartRecommendation, ChartType};
use crate::cli::AggFunction;
use crate::cli::SortOrder;
use crate::render::{Axis, BarChartData, ChartConfig, HistogramData};

pub use ansi::print_buffer;

/// Default chart height in terminal rows.
const DEFAULT_HEIGHT: u16 = 24;

/// Minimum width for chart rendering.
const MIN_WIDTH: u16 = 40;

/// Render a chart to stdout as a one-shot output (no TUI interaction).
/// Options for oneshot rendering.
pub struct RenderOptions<'a> {
    pub chart_type_override: Option<&'a str>,
    pub y_label_override: Option<&'a str>,
    pub width: Option<u16>,
    pub height: Option<u16>,
    pub sort_order: Option<SortOrder>,
    /// Additional Y columns for multi-series overlay (col_name, optional_label).
    pub extra_y_columns: Vec<(String, Option<String>)>,
    /// Limit bar chart to top/tail N categories after sorting.
    pub limit: Option<usize>,
    /// Aggregation function for bar charts.
    pub agg: AggFunction,
    /// Custom chart title (overrides auto-generated title).
    pub title: Option<String>,
}

pub fn render_oneshot(
    recommendation: &ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
    opts: &RenderOptions<'_>,
) -> anyhow::Result<()> {
    let width = opts.width.unwrap_or_else(terminal_width);
    let chart_type = resolve_chart_type(recommendation, opts.chart_type_override);
    let height = opts
        .height
        .unwrap_or_else(|| adaptive_height(chart_type, recommendation, headers, rows));

    summary::print_summary(
        recommendation,
        chart_type,
        headers,
        rows,
        &opts.extra_y_columns,
        opts.agg,
    );

    if opts.sort_order.is_some()
        && opts.sort_order != Some(SortOrder::None)
        && !matches!(chart_type, ChartType::Bar)
    {
        eprintln!(
            "warning: --sort has no effect on {:?} charts (only applies to bar charts)",
            chart_type
        );
    }

    if opts.agg != AggFunction::Sum && !matches!(chart_type, ChartType::Bar) {
        eprintln!(
            "warning: --agg has no effect on {:?} charts (only applies to bar charts)",
            chart_type
        );
    }

    let area = Rect::new(0, 0, width, height);
    let mut buf = Buffer::empty(area);
    render_chart_to_buffer(
        chart_type,
        recommendation,
        headers,
        rows,
        opts,
        area,
        &mut buf,
    );

    print_buffer(&buf, &mut io::stdout().lock())
}

/// Render the appropriate chart type into a buffer.
fn render_chart_to_buffer(
    chart_type: ChartType,
    recommendation: &ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
    opts: &RenderOptions<'_>,
    area: Rect,
    buf: &mut Buffer,
) {
    use crate::render::{ChartData, render_chart_data};

    let chart_data = match chart_type {
        ChartType::Line | ChartType::Scatter => {
            let mut config = build_line_scatter_config(recommendation, headers, rows, opts, area);
            if let Some(ref title) = opts.title {
                config.title = Some(title.clone());
            }
            if chart_type == ChartType::Scatter {
                ChartData::Scatter(config)
            } else {
                ChartData::Line(config)
            }
        }
        ChartType::Heatmap => {
            let mut data = build_heatmap(recommendation, headers, rows);
            if let Some(ref title) = opts.title {
                data.title = Some(title.clone());
            }
            ChartData::Heatmap(data)
        }
        ChartType::Bar => {
            let (mut data, rows_used) = build_bar_data(recommendation, headers, rows, opts.agg);
            if let Some(label) = opts.y_label_override {
                data.y_label = label.to_string();
            }
            if let Some(ref title) = opts.title {
                data.title = Some(title.clone());
            }
            sort_bar_data(&mut data, opts.sort_order);
            truncate_bar_data(&mut data, opts.limit);
            warn_skipped_rows(rows.len(), rows_used, recommendation);
            ChartData::Bar(data)
        }
        ChartType::Histogram => {
            let mut data = build_histogram_data(recommendation, headers, rows);
            if let Some(ref title) = opts.title {
                data.title = Some(title.clone());
            }
            let rendered = data.values.len();
            warn_skipped_rows(rows.len(), rendered, recommendation);
            ChartData::Histogram(data)
        }
    };

    render_chart_data(&chart_data, area, buf);
}

fn build_line_scatter_config(
    recommendation: &ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
    opts: &RenderOptions<'_>,
    area: Rect,
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
        .map(|labels| fit_labels_to_width(&labels, area.width.saturating_sub(12) as usize));
    let rendered = config.series.iter().map(|s| s.data.len()).sum::<usize>();
    let effective_rows = rows.len().min(data_builder::MAX_CHART_POINTS);
    warn_skipped_rows(effective_rows, rendered, recommendation);
    config
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

/// Get terminal width, falling back to 80 columns.
/// When stdout is piped (not a TTY), always returns 80 for deterministic output.
fn terminal_width() -> u16 {
    if !std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        return 80;
    }
    crossterm::terminal::size()
        .map(|(w, _)| w.max(MIN_WIDTH))
        .unwrap_or(80)
}

/// Choose chart height adaptively based on data density.
/// For bar charts with few categories, reduce height to avoid mostly-empty output.
fn adaptive_height(
    chart_type: ChartType,
    recommendation: &ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
) -> u16 {
    if chart_type == ChartType::Bar {
        let x_idx = headers
            .iter()
            .position(|h| h == &recommendation.x_column)
            .unwrap_or(0);
        let unique_categories = rows
            .iter()
            .filter_map(|r| r.get(x_idx))
            .filter(|v| !v.is_empty())
            .collect::<std::collections::HashSet<_>>()
            .len();
        if unique_categories <= 5 {
            // Compact height: at least 10 rows, scale with categories
            return (unique_categories as u16 * 4 + 2).clamp(10, DEFAULT_HEIGHT);
        }
    }
    DEFAULT_HEIGHT
}

/// Reduce label count so labels fit without clipping at the given width.
/// Picks evenly-spaced labels from the input.
fn fit_labels_to_width(labels: &[String], available_width: usize) -> Vec<String> {
    if labels.is_empty() {
        return Vec::new();
    }
    let max_label_len = labels.iter().map(|s| s.len()).max().unwrap_or(1);
    // Each label needs its own length + 1 char padding
    let space_per_label = max_label_len + 1;
    let max_count = (available_width / space_per_label).max(2);
    if labels.len() <= max_count {
        return labels.to_vec();
    }
    data_builder::pick_evenly(labels, max_count)
}

/// Resolve the chart type from recommendation + optional user override.
fn resolve_chart_type(
    recommendation: &ChartRecommendation,
    override_str: Option<&str>,
) -> ChartType {
    match override_str {
        Some(s) => match s.to_lowercase().as_str() {
            "line" => ChartType::Line,
            "bar" => ChartType::Bar,
            "scatter" => ChartType::Scatter,
            "histogram" => ChartType::Histogram,
            "heatmap" => ChartType::Heatmap,
            other => {
                eprintln!(
                    "warning: unknown chart type '{}', using auto. \
                     Valid types: line, bar, scatter, histogram, heatmap",
                    other
                );
                recommendation.chart_type
            }
        },
        None => recommendation.chart_type,
    }
}

/// Print a warning to stderr if rows were skipped during chart building.
fn warn_skipped_rows(
    total_rows: usize,
    rendered_points: usize,
    recommendation: &ChartRecommendation,
) {
    if rendered_points < total_rows {
        let skipped = total_rows - rendered_points;
        let col_name = recommendation
            .y_column
            .as_deref()
            .unwrap_or(&recommendation.x_column);
        eprintln!(
            "warning: {skipped}/{total_rows} rows skipped (non-parseable values in '{col_name}')"
        );
    }
}

/// Resolved column indices and labels from a chart recommendation.
use crate::chart::data_builder::ResolvedAxes;

/// Build a ChartConfig for Line/Scatter charts.
/// When recommendation.color_column is set, splits data into multiple series.
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
fn sort_bar_data(data: &mut BarChartData, sort_order: Option<SortOrder>) {
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
fn truncate_bar_data(data: &mut BarChartData, limit: Option<usize>) {
    if let Some(n) = limit {
        data.labels.truncate(n);
        data.values.truncate(n);
    }
}

/// Build BarChartData for Bar/Heatmap charts.
/// Aggregates values by category using the specified aggregation function.
/// Returns (data, rows_used) where rows_used is the number of rows that contributed.
fn build_bar_data(
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
fn build_histogram_data(
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
    // If x_column is non-numeric, fall back to y_column.
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
fn build_heatmap(
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

/// Find column index by name.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_chart_type_default() {
        let rec = ChartRecommendation {
            chart_type: ChartType::Line,
            x_column: "date".to_string(),
            y_column: Some("revenue".to_string()),
            color_column: None,
        };
        assert_eq!(resolve_chart_type(&rec, None), ChartType::Line);
    }

    #[test]
    fn test_resolve_chart_type_override() {
        let rec = ChartRecommendation {
            chart_type: ChartType::Line,
            x_column: "date".to_string(),
            y_column: Some("revenue".to_string()),
            color_column: None,
        };
        assert_eq!(resolve_chart_type(&rec, Some("bar")), ChartType::Bar);
        assert_eq!(
            resolve_chart_type(&rec, Some("scatter")),
            ChartType::Scatter
        );
        assert_eq!(
            resolve_chart_type(&rec, Some("histogram")),
            ChartType::Histogram
        );
    }

    #[test]
    fn test_resolve_chart_type_invalid_falls_back() {
        let rec = ChartRecommendation {
            chart_type: ChartType::Line,
            x_column: "date".to_string(),
            y_column: Some("revenue".to_string()),
            color_column: None,
        };
        assert_eq!(resolve_chart_type(&rec, Some("invalid")), ChartType::Line);
    }

    #[test]
    fn test_column_index_found() {
        let headers = vec![
            "date".to_string(),
            "city".to_string(),
            "revenue".to_string(),
        ];
        assert_eq!(data_builder::column_index(&headers, "city"), Some(1));
        assert_eq!(data_builder::column_index(&headers, "revenue"), Some(2));
    }

    #[test]
    fn test_column_index_not_found() {
        let headers = vec!["date".to_string(), "city".to_string()];
        assert_eq!(data_builder::column_index(&headers, "nonexistent"), None);
    }

    #[test]
    fn test_build_chart_config_line() {
        let rec = ChartRecommendation {
            chart_type: ChartType::Line,
            x_column: "date".to_string(),
            y_column: Some("revenue".to_string()),
            color_column: None,
        };
        let headers = vec![
            "date".to_string(),
            "city".to_string(),
            "revenue".to_string(),
        ];
        let rows = vec![
            vec![
                "2024-01-01".to_string(),
                "Tokyo".to_string(),
                "1000".to_string(),
            ],
            vec![
                "2024-02-01".to_string(),
                "Osaka".to_string(),
                "1500".to_string(),
            ],
            vec![
                "2024-03-01".to_string(),
                "Tokyo".to_string(),
                "1200".to_string(),
            ],
        ];

        let config = build_chart_config(&rec, &headers, &rows);
        assert_eq!(config.series.len(), 1);
        assert_eq!(config.series[0].data.len(), 3);
        assert!(config.title.unwrap().contains("revenue"));
    }

    #[test]
    fn test_build_bar_data() {
        let rec = ChartRecommendation {
            chart_type: ChartType::Bar,
            x_column: "city".to_string(),
            y_column: Some("revenue".to_string()),
            color_column: None,
        };
        let headers = vec!["city".to_string(), "revenue".to_string()];
        let rows = vec![
            vec!["Tokyo".to_string(), "1000".to_string()],
            vec!["Osaka".to_string(), "1500".to_string()],
        ];

        let (data, rows_used) = build_bar_data(&rec, &headers, &rows, AggFunction::Sum);
        assert_eq!(data.labels, vec!["Tokyo", "Osaka"]);
        assert_eq!(data.values, vec![1000.0, 1500.0]);
        assert_eq!(rows_used, 2);
    }

    #[test]
    fn test_build_bar_data_aggregates_duplicates() {
        let rec = ChartRecommendation {
            chart_type: ChartType::Bar,
            x_column: "city".to_string(),
            y_column: Some("revenue".to_string()),
            color_column: None,
        };
        let headers = vec!["city".to_string(), "revenue".to_string()];
        let rows = vec![
            vec!["Tokyo".to_string(), "1000".to_string()],
            vec!["Osaka".to_string(), "1500".to_string()],
            vec!["Tokyo".to_string(), "2000".to_string()],
            vec!["Osaka".to_string(), "500".to_string()],
        ];

        let (data, rows_used) = build_bar_data(&rec, &headers, &rows, AggFunction::Sum);
        assert_eq!(data.labels, vec!["Tokyo", "Osaka"]);
        assert_eq!(data.values, vec![3000.0, 2000.0]); // Summed
        assert_eq!(rows_used, 4);
    }

    #[test]
    fn test_generate_x_labels_simple() {
        let values: Vec<String> = vec![
            "2024-01-01".to_string(),
            "2024-02-01".to_string(),
            "2024-03-01".to_string(),
        ];
        let labels = data_builder::pick_evenly(&values, 5);
        // When values.len() < count, return all
        assert_eq!(labels, values);
    }

    #[test]
    fn test_generate_x_labels_picks_evenly() {
        let values: Vec<String> = (0..10).map(|i| format!("2024-{:02}-01", i + 1)).collect();
        let labels = data_builder::pick_evenly(&values, 3);
        assert_eq!(labels.len(), 3);
        assert_eq!(labels[0], "2024-01-01"); // first
        assert_eq!(labels[2], "2024-10-01"); // last
    }

    #[test]
    fn test_build_histogram_data() {
        let rec = ChartRecommendation {
            chart_type: ChartType::Histogram,
            x_column: "score".to_string(),
            y_column: None,
            color_column: None,
        };
        let headers = vec!["score".to_string()];
        let rows = vec![
            vec!["85".to_string()],
            vec!["90".to_string()],
            vec!["78".to_string()],
            vec!["92".to_string()],
        ];

        let data = build_histogram_data(&rec, &headers, &rows);
        assert_eq!(data.values.len(), 4);
        assert_eq!(data.bin_count, 10);
        assert_eq!(data.x_label, "score");
    }

    #[test]
    fn test_render_oneshot_line_chart_produces_output() {
        let rec = ChartRecommendation {
            chart_type: ChartType::Line,
            x_column: "date".to_string(),
            y_column: Some("revenue".to_string()),
            color_column: None,
        };
        let headers = vec![
            "date".to_string(),
            "city".to_string(),
            "revenue".to_string(),
        ];
        let rows = vec![
            vec![
                "2024-01-01".to_string(),
                "Tokyo".to_string(),
                "1000".to_string(),
            ],
            vec![
                "2024-02-01".to_string(),
                "Osaka".to_string(),
                "1500".to_string(),
            ],
            vec![
                "2024-03-01".to_string(),
                "Tokyo".to_string(),
                "1200".to_string(),
            ],
            vec![
                "2024-04-01".to_string(),
                "Nagoya".to_string(),
                "800".to_string(),
            ],
            vec![
                "2024-05-01".to_string(),
                "Tokyo".to_string(),
                "2000".to_string(),
            ],
            vec![
                "2024-06-01".to_string(),
                "Osaka".to_string(),
                "1800".to_string(),
            ],
        ];

        // Build the chart config and render to buffer to verify output is non-trivial
        let config = build_chart_config(&rec, &headers, &rows);
        let area = Rect::new(0, 0, 80, 20);
        let mut buf = Buffer::empty(area);
        crate::render::render_chart_data(&crate::render::ChartData::Line(config), area, &mut buf);

        let mut output = Vec::new();
        print_buffer(&buf, &mut output).unwrap();
        let text = String::from_utf8(output).unwrap();

        // Should have multiple lines of output
        assert!(text.lines().count() >= 10);
        // Should contain chart border characters or braille
        assert!(
            text.contains('│') || text.contains('─') || text.contains('┌') || text.contains('⠁')
        );
    }

    #[test]
    fn test_render_oneshot_bar_chart_produces_output() {
        let rec = ChartRecommendation {
            chart_type: ChartType::Bar,
            x_column: "city".to_string(),
            y_column: Some("revenue".to_string()),
            color_column: None,
        };
        let headers = vec!["city".to_string(), "revenue".to_string()];
        let rows = vec![
            vec!["Tokyo".to_string(), "1000".to_string()],
            vec!["Osaka".to_string(), "1500".to_string()],
            vec!["Nagoya".to_string(), "800".to_string()],
        ];

        let (data, _) = build_bar_data(&rec, &headers, &rows, AggFunction::Sum);
        let area = Rect::new(0, 0, 80, 20);
        let mut buf = Buffer::empty(area);
        crate::render::render_chart_data(&crate::render::ChartData::Bar(data), area, &mut buf);

        let mut output = Vec::new();
        print_buffer(&buf, &mut output).unwrap();
        let text = String::from_utf8(output).unwrap();

        assert!(text.lines().count() >= 10);
        // Bar chart should show bar values
        assert!(text.contains("1000") || text.contains("1500") || text.contains("800"));
    }

    #[test]
    fn test_build_grouped_series_shares_x_coordinates() {
        // Simulates sales.csv: each city has data at different row positions
        // but should share the same X coordinate space based on unique X values
        let rows = vec![
            vec![
                "2024-01-01".to_string(),
                "1000".to_string(),
                "Tokyo".to_string(),
            ],
            vec![
                "2024-02-01".to_string(),
                "1500".to_string(),
                "Osaka".to_string(),
            ],
            vec![
                "2024-03-01".to_string(),
                "1200".to_string(),
                "Tokyo".to_string(),
            ],
            vec![
                "2024-04-01".to_string(),
                "800".to_string(),
                "Nagoya".to_string(),
            ],
            vec![
                "2024-05-01".to_string(),
                "2000".to_string(),
                "Tokyo".to_string(),
            ],
            vec![
                "2024-06-01".to_string(),
                "1800".to_string(),
                "Osaka".to_string(),
            ],
        ];

        let series = data_builder::build_grouped_series(&rows, 0, 1, 2, true);

        // Should have 3 groups
        assert_eq!(series.len(), 3);

        let tokyo = series.iter().find(|s| s.name == "Tokyo").unwrap();
        let osaka = series.iter().find(|s| s.name == "Osaka").unwrap();
        let nagoya = series.iter().find(|s| s.name == "Nagoya").unwrap();

        // Tokyo has dates at positions 0, 2, 4 in unique_x order
        assert_eq!(tokyo.data.len(), 3);
        assert_eq!(tokyo.data[0].0, 0.0); // 2024-01-01 → index 0
        assert_eq!(tokyo.data[1].0, 2.0); // 2024-03-01 → index 2
        assert_eq!(tokyo.data[2].0, 4.0); // 2024-05-01 → index 4

        // Osaka has dates at positions 1, 5 in unique_x order
        assert_eq!(osaka.data.len(), 2);
        assert_eq!(osaka.data[0].0, 1.0); // 2024-02-01 → index 1
        assert_eq!(osaka.data[1].0, 5.0); // 2024-06-01 → index 5

        // Nagoya at position 3
        assert_eq!(nagoya.data.len(), 1);
        assert_eq!(nagoya.data[0].0, 3.0); // 2024-04-01 → index 3
    }

    #[test]
    fn test_build_grouped_series_numeric_x() {
        // When X is numeric, use actual numeric values
        let rows = vec![
            vec!["10".to_string(), "100".to_string(), "A".to_string()],
            vec!["20".to_string(), "200".to_string(), "B".to_string()],
            vec!["30".to_string(), "150".to_string(), "A".to_string()],
        ];

        let series = data_builder::build_grouped_series(&rows, 0, 1, 2, false);

        let group_a = series.iter().find(|s| s.name == "A").unwrap();
        assert_eq!(group_a.data[0].0, 10.0);
        assert_eq!(group_a.data[1].0, 30.0);
    }

    #[test]
    fn test_fit_labels_narrow_width() {
        let labels: Vec<String> = vec![
            "2024-01-01".to_string(),
            "2024-02-01".to_string(),
            "2024-03-01".to_string(),
            "2024-04-01".to_string(),
            "2024-05-01".to_string(),
            "2024-06-01".to_string(),
        ];
        // At width 28, each 11-char label needs 11+1=12 chars, so max 2 labels
        let result = fit_labels_to_width(&labels, 28);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "2024-01-01");
        assert_eq!(result[1], "2024-06-01");
    }

    #[test]
    fn test_fit_labels_wide_width() {
        let labels: Vec<String> = vec![
            "Jan".to_string(),
            "Feb".to_string(),
            "Mar".to_string(),
            "Apr".to_string(),
            "May".to_string(),
        ];
        // At width 80, each 4-char label needs 4 chars, so max 20 — all fit
        let result = fit_labels_to_width(&labels, 80);
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_fit_labels_empty() {
        let result = fit_labels_to_width(&[], 80);
        assert!(result.is_empty());
    }

    #[test]
    fn test_sort_bar_data_desc() {
        let mut data = BarChartData {
            labels: vec!["A".into(), "B".into(), "C".into()],
            values: vec![10.0, 30.0, 20.0],
            y_label: String::new(),
            title: None,
        };
        sort_bar_data(&mut data, Some(SortOrder::Desc));
        assert_eq!(data.labels, vec!["B", "C", "A"]);
        assert_eq!(data.values, vec![30.0, 20.0, 10.0]);
    }

    #[test]
    fn test_sort_bar_data_asc() {
        let mut data = BarChartData {
            labels: vec!["A".into(), "B".into(), "C".into()],
            values: vec![10.0, 30.0, 20.0],
            y_label: String::new(),
            title: None,
        };
        sort_bar_data(&mut data, Some(SortOrder::Asc));
        assert_eq!(data.labels, vec!["A", "C", "B"]);
        assert_eq!(data.values, vec![10.0, 20.0, 30.0]);
    }

    #[test]
    fn test_sort_bar_data_none_preserves_order() {
        let mut data = BarChartData {
            labels: vec!["A".into(), "B".into(), "C".into()],
            values: vec![10.0, 30.0, 20.0],
            y_label: String::new(),
            title: None,
        };
        sort_bar_data(&mut data, None);
        assert_eq!(data.labels, vec!["A", "B", "C"]);
        assert_eq!(data.values, vec![10.0, 30.0, 20.0]);
    }

    #[test]
    fn test_sort_bar_data_with_nan() {
        let mut data = BarChartData {
            labels: vec!["A".into(), "B".into(), "C".into()],
            values: vec![f64::NAN, 30.0, 20.0],
            y_label: String::new(),
            title: None,
        };
        // Should not panic with NaN values
        sort_bar_data(&mut data, Some(SortOrder::Desc));
        // NaN.partial_cmp(x) = None → Equal, so stable positioning depends on sort
        // Key assertion: doesn't panic, and non-NaN values remain correctly ordered
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
    fn test_truncate_bar_data() {
        let mut data = BarChartData {
            title: None,
            labels: vec!["A".into(), "B".into(), "C".into()],
            values: vec![100.0, 50.0, 25.0],
            y_label: "val".into(),
        };
        truncate_bar_data(&mut data, Some(2));
        assert_eq!(data.labels, vec!["A", "B"]);
        assert_eq!(data.values, vec![100.0, 50.0]);
    }

    #[test]
    fn test_truncate_bar_data_none() {
        let mut data = BarChartData {
            title: None,
            labels: vec!["A".into(), "B".into(), "C".into()],
            values: vec![100.0, 50.0, 25.0],
            y_label: "val".into(),
        };
        truncate_bar_data(&mut data, None);
        assert_eq!(data.labels.len(), 3);
    }

    #[test]
    fn test_truncate_bar_data_larger_than_data() {
        let mut data = BarChartData {
            title: None,
            labels: vec!["A".into(), "B".into()],
            values: vec![100.0, 50.0],
            y_label: "val".into(),
        };
        truncate_bar_data(&mut data, Some(10));
        assert_eq!(data.labels.len(), 2);
    }

    #[test]
    fn test_adaptive_height_bar_few_categories() {
        let rec = ChartRecommendation {
            chart_type: ChartType::Bar,
            x_column: "city".to_string(),
            y_column: Some("revenue".to_string()),
            color_column: None,
        };
        let headers = vec!["city".to_string(), "revenue".to_string()];
        let rows = vec![
            vec!["Tokyo".to_string(), "1000".to_string()],
            vec!["Osaka".to_string(), "500".to_string()],
        ];
        let height = adaptive_height(ChartType::Bar, &rec, &headers, &rows);
        assert_eq!(height, 10); // 2 * 4 + 2 = 10
    }

    #[test]
    fn test_adaptive_height_bar_many_categories() {
        let rec = ChartRecommendation {
            chart_type: ChartType::Bar,
            x_column: "city".to_string(),
            y_column: Some("revenue".to_string()),
            color_column: None,
        };
        let headers = vec!["city".to_string(), "revenue".to_string()];
        let rows: Vec<Vec<String>> = (0..10)
            .map(|i| vec![format!("City{}", i), "100".to_string()])
            .collect();
        let height = adaptive_height(ChartType::Bar, &rec, &headers, &rows);
        assert_eq!(height, DEFAULT_HEIGHT); // > 5 categories, use default
    }

    #[test]
    fn test_adaptive_height_non_bar() {
        let rec = ChartRecommendation {
            chart_type: ChartType::Line,
            x_column: "date".to_string(),
            y_column: Some("revenue".to_string()),
            color_column: None,
        };
        let headers = vec!["date".to_string(), "revenue".to_string()];
        let rows = vec![vec!["2024-01".to_string(), "100".to_string()]];
        let height = adaptive_height(ChartType::Line, &rec, &headers, &rows);
        assert_eq!(height, DEFAULT_HEIGHT);
    }
}
