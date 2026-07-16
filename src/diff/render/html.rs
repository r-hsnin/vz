//! HTML output for diff mode: renders diff as SVG wrapped in interactive HTML.

use std::path::Path;

use ratatui::{buffer::Buffer, layout::Rect, style::Color};

use crate::cli::Cli;
use crate::diff::{DiffResult, DiffTimeSeries};
use crate::helpers::resolve_theme;
use crate::oneshot::{self, fit_labels_to_width};
use crate::output;
use crate::render::{self, Axis, BarChartData, ChartConfig, ChartData, Series};

use super::apply_sort_and_limit;

/// Render categorical diff as an SVG bar chart wrapped in HTML.
///
/// Bars are colored green (increase) or red (decrease) based on delta direction.
pub(super) fn print_diff_html(cli: &Cli, diff: &DiffResult, before_path: &Path, after_path: &Path) {
    let before_name = before_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("before");
    let after_name = after_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("after");

    let entries = apply_sort_and_limit(cli, &diff.entries);

    let theme = resolve_theme(cli);
    let bg = theme.svg_background();
    let width = cli.width.unwrap_or_else(oneshot::terminal_width);
    let height = cli.height.unwrap_or(24);

    // Color each bar by delta direction: green=increase, red=decrease, gray=unchanged
    let colors: Vec<Color> = entries
        .iter()
        .map(|e| {
            if e.delta > 0.0 {
                Color::Green
            } else if e.delta < 0.0 {
                Color::Red
            } else {
                Color::DarkGray
            }
        })
        .collect();

    let bar_data = BarChartData {
        title: Some(format!("{} vs {}", before_name, after_name)),
        labels: entries.iter().map(|e| e.label.clone()).collect(),
        values: entries.iter().map(|e| e.after).collect(),
        y_label: diff.y_column.clone(),
        show_labels: true,
        series_colors: colors,
        axis_color: Some(Color::DarkGray),
    };

    let area = Rect::new(0, 0, width, height);
    let mut buf = Buffer::empty(area);
    render::render_chart_data(&ChartData::Bar(bar_data), area, &mut buf);

    let svg = output::svg::buffer_to_svg(&buf, bg);
    let title = cli
        .title
        .clone()
        .unwrap_or_else(|| format!("Diff: {} vs {}", before_name, after_name));
    println!("{}", output::html::wrap_svg_in_html(&svg, &title, bg));
}

/// Render temporal diff as a 2-series line chart overlay wrapped in HTML.
///
/// Before series is rendered in gray, after series in cyan.
pub(super) fn print_diff_line_html(
    cli: &Cli,
    ts: &DiffTimeSeries,
    before_path: &Path,
    after_path: &Path,
) {
    let before_name = before_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("before");
    let after_name = after_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("after");

    let theme = resolve_theme(cli);
    let bg = theme.svg_background();
    let width = cli.width.unwrap_or_else(oneshot::terminal_width);
    let height = cli.height.unwrap_or(24);

    // Build Y axis from all values in both series
    let all_y: Vec<f64> = ts
        .before
        .iter()
        .chain(ts.after.iter())
        .map(|(_, y)| *y)
        .collect();
    let y_axis = Axis::from_data(&ts.y_column, &all_y);

    // Build X axis from index range
    let x_max = if ts.x_labels.is_empty() {
        1.0
    } else {
        (ts.x_labels.len() - 1) as f64
    };
    let x_axis = Axis {
        label: ts.x_column.clone(),
        min: 0.0,
        max: x_max,
    };

    // Fit labels to available width
    let fitted_labels = fit_labels_to_width(&ts.x_labels, width.saturating_sub(12) as usize);

    let config = ChartConfig {
        title: Some(
            cli.title
                .clone()
                .unwrap_or_else(|| format!("{} vs {}", before_name, after_name)),
        ),
        x_axis,
        y_axis,
        series: vec![
            Series {
                name: before_name.to_string(),
                data: ts.before.clone(),
            },
            Series {
                name: after_name.to_string(),
                data: ts.after.clone(),
            },
        ],
        x_labels: Some(fitted_labels),
        series_colors: vec![Color::DarkGray, Color::Cyan],
        axis_color: Some(Color::DarkGray),
        label_color: Some(Color::DarkGray),
    };

    let area = Rect::new(0, 0, width, height);
    let mut buf = Buffer::empty(area);
    render::render_chart_data(&ChartData::Line(config), area, &mut buf);

    let svg = output::svg::buffer_to_svg(&buf, bg);
    let title = cli
        .title
        .clone()
        .unwrap_or_else(|| format!("Diff: {} vs {}", before_name, after_name));
    println!("{}", output::html::wrap_svg_in_html(&svg, &title, bg));
}
