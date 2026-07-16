//! Markdown table output for diff mode (both categorical and temporal).

use std::path::Path;

use crate::cli::Cli;
use crate::diff::{DiffResult, DiffTimeSeries};
use crate::render::format_number;

use super::apply_sort_and_limit;

/// Format percentage change as a ▲/▼/─ string.
fn format_change(pct_change: Option<f64>, delta: f64) -> String {
    match pct_change {
        Some(pct) if pct > 0.0 => format!("▲ +{:.0}%", pct),
        Some(pct) if pct < 0.0 => format!("▼ {:.0}%", pct),
        Some(_) => "─ 0%".to_string(),
        None if delta > 0.0 => format!("▲ +{}", format_number(delta)),
        None if delta < 0.0 => format!("▼ {}", format_number(delta)),
        None => "─".to_string(),
    }
}

/// Escape pipe characters in a cell value for valid GFM tables.
fn escape_cell(s: &str) -> String {
    s.replace('|', "\\|")
}

/// Print categorical diff as a Markdown table.
pub(super) fn print_diff_markdown(
    cli: &Cli,
    diff: &DiffResult,
    _before_path: &Path,
    _after_path: &Path,
) {
    let entries = apply_sort_and_limit(cli, &diff.entries);

    let x_col = escape_cell(&diff.x_column);
    println!("| {} | Before | After | Change |", x_col);
    println!("|---|---|---|---|");

    for entry in &entries {
        println!(
            "| {} | {} | {} | {} |",
            escape_cell(&entry.label),
            format_number(entry.before),
            format_number(entry.after),
            format_change(entry.pct_change, entry.delta),
        );
    }

    if let Some(pct) = diff.overall_pct {
        let marker = if pct > 0.0 {
            format!("▲ +{:.0}%", pct)
        } else if pct < 0.0 {
            format!("▼ {:.0}%", pct)
        } else {
            "─ 0%".to_string()
        };
        println!("\n*Overall: {}*", marker);
    }
}

/// Print temporal diff as a Markdown table.
pub(super) fn print_diff_line_markdown(
    ts: &DiffTimeSeries,
    _before_path: &Path,
    _after_path: &Path,
) {
    let x_col = escape_cell(&ts.x_column);
    println!("| {} | Before | After | Change |", x_col);
    println!("|---|---|---|---|");

    // Build aligned rows from before/after series
    for (i, label) in ts.x_labels.iter().enumerate() {
        let before_val = ts
            .before
            .iter()
            .find(|(x, _)| *x as usize == i)
            .map(|(_, y)| *y);
        let after_val = ts
            .after
            .iter()
            .find(|(x, _)| *x as usize == i)
            .map(|(_, y)| *y);

        let (before_str, after_str, change_str) = match (before_val, after_val) {
            (Some(b), Some(a)) => {
                let delta = a - b;
                let pct = if b.abs() > f64::EPSILON {
                    Some((delta / b) * 100.0)
                } else {
                    None
                };
                (
                    format_number(b),
                    format_number(a),
                    format_change(pct, delta),
                )
            }
            (Some(b), None) => (format_number(b), "—".to_string(), "—".to_string()),
            (None, Some(a)) => ("—".to_string(), format_number(a), "▲ new".to_string()),
            (None, None) => ("—".to_string(), "—".to_string(), "—".to_string()),
        };

        println!(
            "| {} | {} | {} | {} |",
            escape_cell(label),
            before_str,
            after_str,
            change_str,
        );
    }

    if let Some(pct) = ts.overall_pct {
        let marker = if pct > 0.0 {
            format!("▲ +{:.0}%", pct)
        } else if pct < 0.0 {
            format!("▼ {:.0}%", pct)
        } else {
            "─ 0%".to_string()
        };
        println!("\n*Overall: {}*", marker);
    }
}
