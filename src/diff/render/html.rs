//! HTML output for diff mode: renders diff as SVG wrapped in interactive HTML.

use std::path::Path;

use ratatui::{buffer::Buffer, layout::Rect, style::Color};

use crate::cli::{DiffParams, resolve_theme_arg};
use crate::diff::{DiffResult, DiffTimeSeries};
use crate::oneshot::{self, fit_labels_to_width};
use crate::output;
use crate::render::{self, ChartData};
use crate::util::path_label;

/// Render categorical diff as an SVG bar chart wrapped in HTML.
///
/// Bars are colored green (increase) or red (decrease) based on delta direction.
pub(super) fn print_diff_html(
    params: &DiffParams,
    diff: &DiffResult,
    before_path: &Path,
    after_path: &Path,
) {
    let before_name = path_label(before_path);
    let after_name = path_label(after_path);

    let theme = resolve_theme_arg(params.theme);
    let bg = theme.svg_background();
    let width = params.width.unwrap_or_else(oneshot::terminal_width);
    let height = params.height.unwrap_or(oneshot::DEFAULT_HEIGHT);

    // Canonical categorical-diff bars (after values + direction-annotated
    // labels); color each bar by delta direction at the edge. Keep the
    // sorted entries aligned so colors match the (possibly sorted) bars.
    let mut tuples: Vec<(String, f64, Option<f64>, f64)> = diff
        .entries
        .iter()
        .map(|e| (e.label.clone(), e.after, e.pct_change, e.delta))
        .collect();
    match params.sort {
        Some(crate::chart::selector::SortOrder::Desc) => {
            tuples.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));
        }
        Some(crate::chart::selector::SortOrder::Asc) => {
            tuples.sort_by(|a, b| a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal));
        }
        _ => {}
    }
    if let Some(n) = params.limit {
        tuples.truncate(n);
    }
    // Color each bar by delta direction: green=increase, red=decrease, gray=unchanged
    let colors: Vec<Color> = tuples
        .iter()
        .map(|(_, _, _, delta)| {
            if *delta > 0.0 {
                Color::Green
            } else if *delta < 0.0 {
                Color::Red
            } else {
                Color::DarkGray
            }
        })
        .collect();

    let mut bar_data = crate::chart::data_builder::build_diff_bar_data(
        &tuples,
        None,
        None,
        diff.y_column.clone(),
        Some(format!("{} vs {}", before_name, after_name)),
    );
    bar_data.series_colors = colors;
    bar_data.axis_color = Some(Color::DarkGray);
    bar_data.show_labels = true;

    let area = Rect::new(0, 0, width, height);
    let mut buf = Buffer::empty(area);
    render::render_chart_data(&ChartData::Bar(bar_data), area, &mut buf);

    let svg = output::svg::buffer_to_svg(&buf, bg);
    let title = params
        .title
        .clone()
        .unwrap_or_else(|| format!("Diff: {} vs {}", before_name, after_name));
    println!("{}", output::html::wrap_svg_in_html(&svg, &title, bg));
}

/// Render temporal diff as a 2-series line chart overlay wrapped in HTML.
///
/// Before series is rendered in gray, after series in cyan.
pub(super) fn print_diff_line_html(
    params: &DiffParams,
    ts: &DiffTimeSeries,
    before_path: &Path,
    after_path: &Path,
) {
    let before_name = path_label(before_path);
    let after_name = path_label(after_path);

    let theme = resolve_theme_arg(params.theme);
    let bg = theme.svg_background();
    let width = params.width.unwrap_or_else(oneshot::terminal_width);
    let height = params.height.unwrap_or(oneshot::DEFAULT_HEIGHT);

    // Build Y axis from all values in both series (canonical assembler);
    // title falls back to the Diff pair.
    let mut config = crate::chart::data_builder::build_diff_line_config(
        &ts.before,
        &ts.after,
        &ts.x_labels,
        &ts.x_column,
        &ts.y_column,
        params.title.clone().or_else(|| {
            Some(format!(
                "{} vs {}",
                path_label(before_path),
                path_label(after_path)
            ))
        }),
    );

    // Fit labels to available width (edge concern; span stays on the
    // full union set built by the canonical assembler).
    config.x_labels = Some(fit_labels_to_width(
        &ts.x_labels,
        width.saturating_sub(12) as usize,
    ));
    config.series[0].name = before_name.to_string();
    config.series[1].name = after_name.to_string();

    let area = Rect::new(0, 0, width, height);
    let mut buf = Buffer::empty(area);
    render::render_chart_data(&ChartData::Line(config), area, &mut buf);

    let svg = output::svg::buffer_to_svg(&buf, bg);
    let title = params
        .title
        .clone()
        .unwrap_or_else(|| format!("Diff: {} vs {}", before_name, after_name));
    println!("{}", output::html::wrap_svg_in_html(&svg, &title, bg));
}
