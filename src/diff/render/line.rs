//! Temporal line chart overlay rendering for diff output.

use anyhow::Result;
use std::io;
use std::path::Path;

use ratatui::{buffer::Buffer, layout::Rect};

use crate::cli::DiffParams;
use crate::diff::DiffTimeSeries;
use crate::oneshot::{self, fit_labels_to_width};
use crate::render::{self, ChartData};
use crate::util::path_label;

/// Print temporal diff summary: `Line │ x=date │ before vs after │ Δ +N% │ 6 rows`
pub(super) fn print_diff_line_summary(ts: &DiffTimeSeries, before_path: &Path, after_path: &Path) {
    let before_name = path_label(before_path);
    let after_name = path_label(after_path);

    let overall = match ts.overall_pct {
        Some(pct) if pct > 0.0 => format!("Δ +{:.0}%", pct),
        Some(pct) if pct < 0.0 => format!("Δ {:.0}%", pct),
        Some(_) => "Δ 0%".to_string(),
        None => "Δ new".to_string(),
    };

    let total_rows = ts.x_labels.len();
    eprintln!(
        "Line │ x={} │ {} vs {} │ {} │ {} rows",
        ts.x_column, before_name, after_name, overall, total_rows,
    );
}

/// Render the line chart overlay into a buffer and print to stdout.
pub(super) fn print_diff_line_chart(
    params: &DiffParams,
    ts: &DiffTimeSeries,
    before_path: &Path,
    after_path: &Path,
) -> Result<()> {
    let before_name = path_label(before_path);
    let after_name = path_label(after_path);

    // Canonical temporal-diff config (full union labels drive the X
    // span); fit labels to terminal width at the edge, then swap in the
    // fitted subset for display.
    let width = params.width.unwrap_or_else(oneshot::terminal_width);
    let height = params.height.unwrap_or(oneshot::DEFAULT_HEIGHT);
    let mut config = crate::chart::data_builder::build_diff_line_config(
        &ts.before,
        &ts.after,
        &ts.x_labels,
        &ts.x_column,
        &ts.y_column,
        Some(format!("{} vs {}", before_name, after_name)),
    );
    config.x_labels = Some(fit_labels_to_width(
        &ts.x_labels,
        width.saturating_sub(12) as usize,
    ));
    config.series[0].name = before_name.to_string();
    config.series[1].name = after_name.to_string();

    let area = Rect::new(0, 0, width, height);
    let mut buf = Buffer::empty(area);
    render::render_chart_data(&ChartData::Line(config), area, &mut buf);
    oneshot::print_buffer(&buf, &mut io::stdout().lock())
}
