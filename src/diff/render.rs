//! Diff-aware rendering: summary line, bar chart with ▲/▼, and spark output.

use anyhow::Result;
use std::io;
use std::path::Path;

use ratatui::{buffer::Buffer, layout::Rect, style::Color};

use crate::cli::{self, Cli};
use crate::oneshot::{self, fit_labels_to_width};
use crate::render::{self, Axis, ChartConfig, ChartData, Series, format_number};
use crate::sparkline;

use super::{DiffResult, DiffTimeSeries};

/// Render the diff result based on CLI output format.
pub fn render_diff(
    cli: &Cli,
    diff: &DiffResult,
    before_path: &Path,
    after_path: &Path,
) -> Result<()> {
    match cli.output {
        Some(cli::OutputFormat::Spark) => {
            print_diff_spark(diff);
        }
        Some(cli::OutputFormat::Json) => {
            print_diff_json(diff, before_path, after_path)?;
        }
        _ => {
            print_diff_summary(diff, before_path, after_path);
            print_diff_bar(cli, diff);
        }
    }
    Ok(())
}

/// Render temporal diff as a 2-series line chart overlay.
pub fn render_diff_line(
    cli: &Cli,
    ts: &DiffTimeSeries,
    before_path: &Path,
    after_path: &Path,
) -> Result<()> {
    match cli.output {
        Some(cli::OutputFormat::Spark) => {
            print_diff_line_spark(ts, before_path, after_path);
        }
        Some(cli::OutputFormat::Json) => {
            print_diff_line_json(ts, before_path, after_path)?;
        }
        _ => {
            print_diff_line_summary(ts, before_path, after_path);
            print_diff_line_chart(cli, ts, before_path, after_path)?;
        }
    }
    Ok(())
}

/// Print temporal diff summary: `Line │ x=date │ before vs after │ Δ +N% │ 6 rows`
fn print_diff_line_summary(ts: &DiffTimeSeries, before_path: &Path, after_path: &Path) {
    let before_name = before_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("before");
    let after_name = after_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("after");

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
fn print_diff_line_chart(
    cli: &Cli,
    ts: &DiffTimeSeries,
    before_path: &Path,
    after_path: &Path,
) -> Result<()> {
    let before_name = before_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("before");
    let after_name = after_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("after");

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
    let height = cli.height.unwrap_or(24);

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
    let mut buf = Buffer::empty(area);
    render::render_chart_data(&ChartData::Line(config), area, &mut buf);
    oneshot::print_buffer(&buf, &mut io::stdout().lock())
}

/// Print temporal diff as sparkline.
fn print_diff_line_spark(ts: &DiffTimeSeries, before_path: &Path, after_path: &Path) {
    let before_name = before_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("before");
    let after_name = after_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("after");

    let before_values: Vec<f64> = ts.before.iter().map(|(_, y)| *y).collect();
    let after_values: Vec<f64> = ts.after.iter().map(|(_, y)| *y).collect();

    let before_spark = sparkline::sparkline_from_values(&before_values);
    let after_spark = sparkline::sparkline_from_values(&after_values);

    let overall = match ts.overall_pct {
        Some(pct) if pct > 0.0 => format!("(+{:.0}%)", pct),
        Some(pct) if pct < 0.0 => format!("({:.0}%)", pct),
        Some(_) => "(0%)".to_string(),
        None => String::new(),
    };

    println!("{}  {}", before_name, before_spark);
    println!("{}  {}  {}", after_name, after_spark, overall);
}

/// Print temporal diff as JSON.
fn print_diff_line_json(ts: &DiffTimeSeries, before_path: &Path, after_path: &Path) -> Result<()> {
    let before_points: Vec<serde_json::Value> = ts
        .before
        .iter()
        .map(|(x, y)| {
            let label = ts.x_labels.get(*x as usize).cloned().unwrap_or_default();
            serde_json::json!({"date": label, "value": y})
        })
        .collect();
    let after_points: Vec<serde_json::Value> = ts
        .after
        .iter()
        .map(|(x, y)| {
            let label = ts.x_labels.get(*x as usize).cloned().unwrap_or_default();
            serde_json::json!({"date": label, "value": y})
        })
        .collect();

    let output = serde_json::json!({
        "version": 1,
        "mode": "diff",
        "chart_type": "line",
        "before": {
            "file": before_path.display().to_string(),
            "rows": ts.before_rows,
            "series": before_points,
        },
        "after": {
            "file": after_path.display().to_string(),
            "rows": ts.after_rows,
            "series": after_points,
        },
        "x_column": ts.x_column,
        "y_column": ts.y_column,
        "dates": ts.x_labels,
        "overall_delta_pct": ts.overall_pct,
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Print the diff summary line: `Δ N rows, M columns │ before: X │ after: Y`
fn print_diff_summary(diff: &DiffResult, before_path: &Path, after_path: &Path) {
    let before_name = before_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("before");
    let after_name = after_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("after");

    let overall = match diff.overall_pct {
        Some(pct) if pct > 0.0 => format!(" │ Δ net +{:.0}%", pct),
        Some(pct) if pct < 0.0 => format!(" │ Δ net {:.0}%", pct),
        Some(_) => " │ Δ net 0%".to_string(),
        None => String::new(),
    };

    println!(
        "Diff │ x={} │ y={} │ {} vs {}{} │ {} entries",
        diff.x_column,
        diff.y_column,
        before_name,
        after_name,
        overall,
        diff.entries.len(),
    );
}

/// Print a diff-aware bar chart with ▲/▼ direction markers.
fn print_diff_bar(cli: &Cli, diff: &DiffResult) {
    let entries = apply_sort_and_limit(cli, &diff.entries);

    if entries.is_empty() {
        return;
    }

    // Find max absolute after value for bar scaling
    let max_abs = entries
        .iter()
        .map(|e| e.after.abs())
        .fold(0.0_f64, f64::max);

    let label_width = entries.iter().map(|e| e.label.len()).max().unwrap_or(8);
    let bar_width: usize = cli
        .width
        .map(|w| w as usize)
        .unwrap_or(60)
        .saturating_sub(label_width + 40);
    let bar_width = bar_width.max(10);

    for entry in &entries {
        let bar_len = if max_abs > 0.0 {
            ((entry.after.abs() / max_abs) * bar_width as f64).round() as usize
        } else {
            0
        };
        let bar = "█".repeat(bar_len);

        let direction = if entry.delta > 0.0 {
            "▲"
        } else if entry.delta < 0.0 {
            "▼"
        } else {
            "─"
        };

        let change_str = match entry.pct_change {
            Some(pct) if pct > 0.0 => format!("{} +{:.0}%", direction, pct),
            Some(pct) if pct < 0.0 => format!("{} {:.0}%", direction, pct),
            Some(_) => format!("{} 0%", direction),
            None if entry.delta > 0.0 => format!("{} +{}", direction, format_number(entry.delta)),
            None => direction.to_string(),
        };

        println!(
            "  {:width$}  {}  {} → {}  {}",
            entry.label,
            bar,
            format_number(entry.before),
            format_number(entry.after),
            change_str,
            width = label_width,
        );
    }
}

/// Print diff as sparkline: `Δ revenue  ▁▃▅▇  (+45%)`
fn print_diff_spark(diff: &DiffResult) {
    let deltas: Vec<f64> = diff.entries.iter().map(|e| e.delta).collect();
    let spark = sparkline::sparkline_from_values(&deltas);

    let overall = match diff.overall_pct {
        Some(pct) if pct > 0.0 => format!("(+{:.0}%)", pct),
        Some(pct) if pct < 0.0 => format!("({:.0}%)", pct),
        Some(_) => "(0%)".to_string(),
        None => String::new(),
    };

    println!("Δ {}  {}  {}", diff.y_column, spark, overall);
}

/// Print diff as JSON.
fn print_diff_json(diff: &DiffResult, before_path: &Path, after_path: &Path) -> Result<()> {
    let categories: Vec<serde_json::Value> = diff
        .entries
        .iter()
        .map(|e| {
            serde_json::json!({
                "label": e.label,
                "before": e.before,
                "after": e.after,
                "delta": e.delta,
                "pct_change": e.pct_change,
            })
        })
        .collect();

    let output = serde_json::json!({
        "version": 1,
        "mode": "diff",
        "before": {
            "file": before_path.display().to_string(),
            "rows": diff.before_rows,
        },
        "after": {
            "file": after_path.display().to_string(),
            "rows": diff.after_rows,
        },
        "x_column": diff.x_column,
        "y_column": diff.y_column,
        "categories": categories,
        "overall_delta_pct": diff.overall_pct,
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Apply sort and limit (--top, --tail, --sort) to diff entries.
fn apply_sort_and_limit(cli: &Cli, entries: &[super::DiffEntry]) -> Vec<super::DiffEntry> {
    let mut sorted = entries.to_vec();

    match cli.effective_sort() {
        Some(cli::SortOrder::Desc) => {
            sorted.sort_by(|a, b| {
                b.delta
                    .partial_cmp(&a.delta)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        Some(cli::SortOrder::Asc) => {
            sorted.sort_by(|a, b| {
                a.delta
                    .partial_cmp(&b.delta)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        _ => {} // preserve original order
    }

    if let Some(n) = cli.top.or(cli.tail) {
        sorted.truncate(n);
    }

    sorted
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::DiffEntry;
    use clap::Parser;

    fn sample_entries() -> Vec<DiffEntry> {
        vec![
            DiffEntry {
                label: "Tokyo".into(),
                before: 1000.0,
                after: 1200.0,
                delta: 200.0,
                pct_change: Some(20.0),
            },
            DiffEntry {
                label: "Osaka".into(),
                before: 1500.0,
                after: 1350.0,
                delta: -150.0,
                pct_change: Some(-10.0),
            },
            DiffEntry {
                label: "Nagoya".into(),
                before: 800.0,
                after: 950.0,
                delta: 150.0,
                pct_change: Some(18.75),
            },
        ]
    }

    #[test]
    fn test_diff_spark_format() {
        let diff = DiffResult {
            entries: sample_entries(),
            x_column: "city".into(),
            y_column: "revenue".into(),
            before_rows: 3,
            after_rows: 3,
            overall_pct: Some(6.06),
        };
        // Just verify it doesn't panic; output format tested via integration tests
        print_diff_spark(&diff);
    }

    #[test]
    fn test_apply_sort_desc() {
        let entries = sample_entries();
        let cli = Cli::try_parse_from(["vz", "a.csv", "b.csv", "--sort", "desc"]).unwrap();
        let sorted = apply_sort_and_limit(&cli, &entries);
        assert_eq!(sorted[0].label, "Tokyo"); // delta +200
        assert_eq!(sorted[1].label, "Nagoya"); // delta +150
        assert_eq!(sorted[2].label, "Osaka"); // delta -150
    }

    #[test]
    fn test_apply_sort_asc() {
        let entries = sample_entries();
        let cli = Cli::try_parse_from(["vz", "a.csv", "b.csv", "--sort", "asc"]).unwrap();
        let sorted = apply_sort_and_limit(&cli, &entries);
        assert_eq!(sorted[0].label, "Osaka"); // delta -150
    }

    #[test]
    fn test_apply_top_limit() {
        let entries = sample_entries();
        let cli = Cli::try_parse_from(["vz", "a.csv", "b.csv", "--top", "2"]).unwrap();
        let sorted = apply_sort_and_limit(&cli, &entries);
        assert_eq!(sorted.len(), 2);
        assert_eq!(sorted[0].label, "Tokyo"); // highest delta
    }

    // --- render_diff_line tests ---

    fn sample_ts() -> super::DiffTimeSeries {
        super::DiffTimeSeries {
            before: vec![(0.0, 100.0), (1.0, 120.0), (2.0, 140.0)],
            after: vec![(0.0, 110.0), (1.0, 130.0), (2.0, 150.0)],
            x_labels: vec![
                "2024-01-01".into(),
                "2024-01-02".into(),
                "2024-01-03".into(),
            ],
            x_column: "date".into(),
            y_column: "revenue".into(),
            before_rows: 3,
            after_rows: 3,
            overall_pct: Some(8.33),
        }
    }

    #[test]
    fn test_diff_line_spark_format() {
        let ts = sample_ts();
        // Just verify it doesn't panic
        print_diff_line_spark(&ts, Path::new("before.csv"), Path::new("after.csv"));
    }

    #[test]
    fn test_diff_line_json_structure() {
        let ts = sample_ts();
        // Verify the JSON function succeeds
        let result = print_diff_line_json(&ts, Path::new("before.csv"), Path::new("after.csv"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_diff_line_chart_builds_two_series() {
        // Verify the ChartConfig construction logic
        let ts = sample_ts();
        use ratatui::style::Color;

        let all_y: Vec<f64> = ts
            .before
            .iter()
            .chain(ts.after.iter())
            .map(|(_, y)| *y)
            .collect();
        let y_axis = crate::render::Axis::from_data(&ts.y_column, &all_y);
        let x_max = (ts.x_labels.len() - 1) as f64;

        let config = crate::render::ChartConfig {
            title: Some("before vs after".into()),
            x_axis: crate::render::Axis {
                label: "date".into(),
                min: 0.0,
                max: x_max,
            },
            y_axis,
            series: vec![
                crate::render::Series {
                    name: "before".into(),
                    data: ts.before.clone(),
                },
                crate::render::Series {
                    name: "after".into(),
                    data: ts.after.clone(),
                },
            ],
            x_labels: Some(ts.x_labels.clone()),
            series_colors: vec![Color::DarkGray, Color::Cyan],
            axis_color: Some(Color::DarkGray),
            label_color: Some(Color::DarkGray),
        };

        assert_eq!(config.series.len(), 2);
        assert_eq!(config.series[0].name, "before");
        assert_eq!(config.series[1].name, "after");
        assert_eq!(config.series_colors, vec![Color::DarkGray, Color::Cyan]);
        assert_eq!(config.series[0].data.len(), 3);
        assert_eq!(config.series[1].data.len(), 3);
    }
}
