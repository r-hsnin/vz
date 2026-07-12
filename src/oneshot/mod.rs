pub mod ansi;
mod builders;
mod summary;

use std::io;

use ratatui::{buffer::Buffer, layout::Rect};

use crate::chart::data_builder;
use crate::chart::selector::{ChartRecommendation, ChartType};
use crate::cli::AggFunction;
use crate::cli::SortOrder;

pub use ansi::print_buffer;

/// Default chart height in terminal rows.
const DEFAULT_HEIGHT: u16 = 24;

/// Minimum width for chart rendering.
const MIN_WIDTH: u16 = 40;

/// Render a chart to stdout as a one-shot output (no TUI interaction).
/// Options for oneshot rendering.
pub struct RenderOptions<'a> {
    pub chart_type_override: Option<crate::cli::ChartTypeArg>,
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
    /// Show value + percentage labels on bar chart bars.
    pub labels: bool,
    /// Color theme for rendering.
    pub theme: crate::theme::Theme,
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

    // For bar charts, compute post-aggregation stats so the summary line
    // shows the actual rendered values (e.g. sums), not the raw per-row values.
    let agg_stats = if chart_type == ChartType::Bar {
        compute_bar_agg_stats(recommendation, headers, rows, opts.agg)
    } else {
        None
    };

    summary::print_summary(
        recommendation,
        chart_type,
        headers,
        rows,
        &opts.extra_y_columns,
        opts.agg,
        agg_stats,
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

    if opts.labels && !matches!(chart_type, ChartType::Bar) {
        eprintln!(
            "warning: --labels has no effect on {:?} charts (only applies to bar charts)",
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

/// Compute min/max of aggregated bar chart values (post-sum/mean/etc).
fn compute_bar_agg_stats(
    recommendation: &ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
    agg: AggFunction,
) -> Option<(f64, f64)> {
    let x_idx = headers.iter().position(|h| h == &recommendation.x_column)?;
    let y_col = recommendation.y_column.as_ref()?;
    let y_idx = headers.iter().position(|h| h == y_col)?;
    let y_label = y_col.clone();
    let (data, _) = data_builder::aggregate_bar(rows, x_idx, y_idx, None, y_label, agg);
    if data.values.is_empty() {
        return None;
    }
    let min = data.values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = data
        .values
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    Some((min, max))
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

    let mut chart_data = match chart_type {
        ChartType::Line | ChartType::Scatter => {
            let config = builders::build_line_scatter_config(
                recommendation,
                headers,
                rows,
                opts,
                area,
                chart_type,
            );
            if chart_type == ChartType::Scatter {
                ChartData::Scatter(config)
            } else {
                ChartData::Line(config)
            }
        }
        ChartType::Heatmap => {
            let data = builders::build_heatmap(recommendation, headers, rows);
            ChartData::Heatmap(data)
        }
        ChartType::Bar => {
            let (mut data, rows_used) =
                builders::build_bar_data(recommendation, headers, rows, opts.agg);
            if let Some(label) = opts.y_label_override {
                data.y_label = label.to_string();
            }
            data.show_labels = opts.labels;
            data.series_colors = opts.theme.series_colors.clone();
            builders::sort_bar_data(&mut data, opts.sort_order);
            builders::truncate_bar_data(&mut data, opts.limit);
            warn_skipped_rows(rows.len(), rows_used, recommendation, ChartType::Bar);
            ChartData::Bar(data)
        }
        ChartType::Histogram => {
            let data = builders::build_histogram_data(recommendation, headers, rows);
            let rendered = data.values.len();
            warn_skipped_rows(rows.len(), rendered, recommendation, ChartType::Histogram);
            ChartData::Histogram(data)
        }
    };

    // Apply title override once for all chart types
    if let Some(ref title) = opts.title {
        chart_data.set_title(title.clone());
    }

    render_chart_data(&chart_data, area, buf);
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
fn adaptive_height(
    chart_type: ChartType,
    recommendation: &ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
) -> u16 {
    match chart_type {
        ChartType::Bar | ChartType::Heatmap => {
            let x_idx = headers
                .iter()
                .position(|h| h == &recommendation.x_column)
                .unwrap_or(0);
            let unique: std::collections::HashSet<&str> = rows
                .iter()
                .filter_map(|r| r.get(x_idx).map(|s| s.as_str()))
                .collect();
            if unique.len() <= 5 {
                ((unique.len() as u16) * 4 + 2).clamp(10, DEFAULT_HEIGHT)
            } else {
                DEFAULT_HEIGHT
            }
        }
        ChartType::Line | ChartType::Scatter => {
            if rows.len() <= 6 {
                ((rows.len() as u16) * 3 + 6).clamp(12, DEFAULT_HEIGHT)
            } else {
                DEFAULT_HEIGHT
            }
        }
        _ => DEFAULT_HEIGHT,
    }
}

/// Fit labels to available width by selecting evenly-spaced subset.
pub(crate) fn fit_labels_to_width(labels: &[String], available_width: usize) -> Vec<String> {
    if labels.is_empty() {
        return vec![];
    }
    let max_label_width = labels.iter().map(|l| l.len()).max().unwrap_or(1);
    let labels_that_fit = (available_width / (max_label_width + 2)).max(2);
    if labels.len() <= labels_that_fit {
        return labels.to_vec();
    }
    pick_evenly(labels, labels_that_fit)
}

/// Pick n items evenly spaced from a slice, always including first and last.
fn pick_evenly(items: &[String], n: usize) -> Vec<String> {
    if n >= items.len() {
        return items.to_vec();
    }
    let step = (items.len() - 1) as f64 / (n - 1) as f64;
    (0..n)
        .map(|i| items[(i as f64 * step).round() as usize].clone())
        .collect()
}

/// Resolve the chart type: use override if given, otherwise use the recommended type.
pub fn resolve_chart_type(
    recommendation: &ChartRecommendation,
    override_type: Option<crate::cli::ChartTypeArg>,
) -> ChartType {
    if let Some(ct) = override_type {
        ct.to_chart_type()
    } else {
        recommendation.chart_type
    }
}

pub(crate) fn warn_skipped_rows(
    total_rows: usize,
    rendered_rows: usize,
    recommendation: &ChartRecommendation,
    chart_type: ChartType,
) {
    if rendered_rows < total_rows && total_rows > 0 {
        let skipped = total_rows - rendered_rows;
        let pct = (skipped as f64 / total_rows as f64) * 100.0;
        if pct > 10.0 {
            let blame_col = match chart_type {
                ChartType::Bar | ChartType::Heatmap => &recommendation.x_column,
                _ => recommendation
                    .y_column
                    .as_deref()
                    .unwrap_or(&recommendation.x_column),
            };
            eprintln!(
                "warning: {} of {} rows ({:.0}%) have non-parseable values in column '{}' and were skipped",
                skipped, total_rows, pct, blame_col
            );
        }
    }
}

// Re-export builder functions for use in tests
#[cfg(test)]
use crate::render::{BarChartData, ChartConfig};
#[cfg(test)]
use builders::{build_bar_data, build_histogram_data, sort_bar_data, truncate_bar_data};

/// Build ChartConfig for line/scatter charts (used by tests).
#[cfg(test)]
fn build_chart_config(
    recommendation: &ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
) -> ChartConfig {
    let axes = data_builder::ResolvedAxes::from_recommendation(
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
        assert_eq!(
            resolve_chart_type(&rec, Some(crate::cli::ChartTypeArg::Bar)),
            ChartType::Bar
        );
        assert_eq!(
            resolve_chart_type(&rec, Some(crate::cli::ChartTypeArg::Scatter)),
            ChartType::Scatter
        );
        assert_eq!(
            resolve_chart_type(&rec, Some(crate::cli::ChartTypeArg::Histogram)),
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
        // With ValueEnum, invalid types are rejected at parse time by clap.
        // None falls back to recommendation.
        assert_eq!(resolve_chart_type(&rec, None), ChartType::Line);
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
            show_labels: false,
            series_colors: vec![],
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
            show_labels: false,
            series_colors: vec![],
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
            show_labels: false,
            series_colors: vec![],
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
            show_labels: false,
            series_colors: vec![],
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
            show_labels: false,
            series_colors: vec![],
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
            show_labels: false,
            series_colors: vec![],
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
            show_labels: false,
            series_colors: vec![],
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
        // 1 row → adaptive: 1*3+6=9, clamped to min 12
        let rows = vec![vec!["2024-01".to_string(), "100".to_string()]];
        let height = adaptive_height(ChartType::Line, &rec, &headers, &rows);
        assert_eq!(height, 12);
    }
}

#[test]
fn test_adaptive_height_line_small_dataset() {
    let rec = ChartRecommendation {
        chart_type: ChartType::Line,
        x_column: "date".to_string(),
        y_column: Some("value".to_string()),
        color_column: None,
    };
    let headers = vec!["date".to_string(), "value".to_string()];
    // 3 rows → height = 3*3+6 = 15
    let rows = vec![
        vec!["2024-01".into(), "10".into()],
        vec!["2024-02".into(), "20".into()],
        vec!["2024-03".into(), "30".into()],
    ];
    let height = adaptive_height(ChartType::Line, &rec, &headers, &rows);
    assert_eq!(height, 15);
}

#[test]
fn test_adaptive_height_scatter_small_dataset() {
    let rec = ChartRecommendation {
        chart_type: ChartType::Scatter,
        x_column: "x".to_string(),
        y_column: Some("y".to_string()),
        color_column: None,
    };
    let headers = vec!["x".to_string(), "y".to_string()];
    // 2 rows → height = 2*3+6 = 12
    let rows = vec![vec!["1".into(), "2".into()], vec!["3".into(), "4".into()]];
    let height = adaptive_height(ChartType::Scatter, &rec, &headers, &rows);
    assert_eq!(height, 12);
}

#[test]
fn test_adaptive_height_line_large_dataset_uses_default() {
    let rec = ChartRecommendation {
        chart_type: ChartType::Line,
        x_column: "date".to_string(),
        y_column: Some("value".to_string()),
        color_column: None,
    };
    let headers = vec!["date".to_string(), "value".to_string()];
    // 10 rows → exceeds threshold, uses DEFAULT_HEIGHT
    let rows: Vec<Vec<String>> = (0..10)
        .map(|i| vec![format!("2024-{:02}", i + 1), format!("{}", i * 10)])
        .collect();
    let height = adaptive_height(ChartType::Line, &rec, &headers, &rows);
    assert_eq!(height, DEFAULT_HEIGHT);
}
