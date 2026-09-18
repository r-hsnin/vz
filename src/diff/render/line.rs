//! Temporal line chart overlay rendering for diff output.

use anyhow::Result;
use std::io;
use std::path::Path;

use ratatui::{buffer::Buffer, layout::Rect, style::Color};

use crate::cli::Cli;
use crate::diff::DiffTimeSeries;
use crate::oneshot::{self, fit_labels_to_width};
use crate::render::{self, Axis, ChartConfig, ChartData, Series};
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
    cli: &Cli,
    ts: &DiffTimeSeries,
    before_path: &Path,
    after_path: &Path,
) -> Result<()> {
    let before_name = path_label(before_path);
    let after_name = path_label(after_path);

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

    let width = cli.width.unwrap_or_else(oneshot::terminal_width);
    let height = cli.height.unwrap_or(oneshot::DEFAULT_HEIGHT);

    // Fit labels to available width
    let fitted_labels = fit_labels_to_width(&ts.x_labels, width.saturating_sub(12) as usize);

    let config = ChartConfig {
        title: Some(format!("{} vs {}", before_name, after_name)),
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
    let final_chart = ChartData::Line(config);

    // Animated draw (TTY only): series reveal left-to-right on the final
    // axes; piped output and --motion off render the final frame once.
    let motion = crate::anim::MotionConfig::new(cli.motion, cli.fps, cli.frames);
    let no_color = std::env::var("NO_COLOR").is_ok_and(|v| !v.is_empty());
    if crate::anim::should_animate(
        &motion,
        std::io::IsTerminal::is_terminal(&std::io::stdout()),
        no_color,
    ) && let Some(effect) =
        crate::anim::resolve_effect(cli.motion, crate::chart::selector::ChartType::Line)
    {
        let frames = crate::anim::build_frames(&final_chart, Some(effect), motion.frames as usize);
        let delay = std::time::Duration::from_secs_f64(1.0 / f64::from(motion.fps));
        if crate::oneshot::render_animated(&frames, area, delay).is_ok() {
            return Ok(());
        }
        // Fall through to static on player error.
    }

    let mut buf = Buffer::empty(area);
    render::render_chart_data(&final_chart, area, &mut buf);
    oneshot::print_buffer(&buf, &mut io::stdout().lock())
}
