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

    let skipped_rows = count_skipped_y_rows(recommendation, headers, rows);

    summary::print_summary(&summary::SummaryContext {
        recommendation,
        chart_type,
        headers,
        rows,
        extra_y_columns: &opts.extra_y_columns,
        agg: opts.agg,
        agg_stats,
        skipped_rows,
        series_colors: &opts.theme.series_colors,
    });

    warn_incompatible_flags(chart_type, opts);

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

/// Emit warnings when CLI flags are used with incompatible chart types.
fn warn_incompatible_flags(chart_type: ChartType, opts: &RenderOptions<'_>) {
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
}

/// Compute min/max of aggregated bar chart values (post-sum/mean/etc).
fn compute_bar_agg_stats(
    recommendation: &ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
    agg: AggFunction,
) -> Option<(f64, f64)> {
    let x_idx = data_builder::column_index(headers, &recommendation.x_column)?;
    let y_col = recommendation.y_column.as_ref()?;
    let y_idx = data_builder::column_index(headers, y_col)?;
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
            data.axis_color = Some(opts.theme.axis_color);
            builders::sort_bar_data(&mut data, opts.sort_order);
            builders::truncate_bar_data(&mut data, opts.limit);
            warn_skipped_rows(rows.len(), rows_used, recommendation, ChartType::Bar);
            ChartData::Bar(data)
        }
        ChartType::Histogram => {
            let mut data = builders::build_histogram_data(recommendation, headers, rows);
            data.axis_color = Some(opts.theme.axis_color);
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
    // Small datasets: always show all labels (avoids confusing elision)
    if labels.len() <= 10 {
        return labels.to_vec();
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

/// Count rows that would be skipped due to non-parseable Y values.
/// Used by the summary line to show "(N skipped)".
fn count_skipped_y_rows(
    recommendation: &ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
) -> usize {
    let y_idx = recommendation
        .y_column
        .as_deref()
        .and_then(|name| data_builder::column_index(headers, name));
    let Some(idx) = y_idx else {
        return 0;
    };
    rows.iter()
        .filter(|row| {
            row.get(idx)
                .map(|v| v.trim().parse::<f64>().is_err())
                .unwrap_or(true)
        })
        .count()
}

// Re-export builder functions for use in tests
#[cfg(test)]
use crate::render::ChartConfig;
#[cfg(test)]
use builders::{build_bar_data, build_histogram_data};

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

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
