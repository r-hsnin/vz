//! Diff-aware rendering: summary line, bar chart with ▲/▼, and spark output.

use anyhow::Result;
use std::path::Path;

use crate::cli::{self, Cli};
use crate::render::format_number;
use crate::sparkline;

use super::DiffResult;

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
}
