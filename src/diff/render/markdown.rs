//! Markdown table output for diff mode (both categorical and temporal).

use std::path::Path;

use crate::cli::DiffParams;
use crate::diff::{DiffResult, DiffTimeSeries};
use crate::render::format_number;

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
    params: &DiffParams,
    diff: &DiffResult,
    _before_path: &Path,
    _after_path: &Path,
) {
    // Canonical sort/limit + direction labels; this edge keeps only the
    // Markdown table layout.
    let tuples: Vec<(String, f64, Option<f64>, f64)> = diff
        .entries
        .iter()
        .map(|e| (e.label.clone(), e.after, e.pct_change, e.delta))
        .collect();
    let data = crate::chart::data_builder::build_diff_bar_data(
        &tuples,
        params.sort,
        params.limit,
        diff.y_column.clone(),
        None,
    );
    let by_label: std::collections::HashMap<&str, &crate::diff::DiffEntry> =
        diff.entries.iter().map(|e| (e.label.as_str(), e)).collect();

    let x_col = escape_cell(&diff.x_column);
    println!("| {} | Before | After | Change |", x_col);
    println!("|---|---|---|---|");

    for (label_with_change, after) in data.labels.iter().zip(data.values.iter()) {
        let mut parts = label_with_change.rsplitn(3, ' ');
        let change = parts.next().unwrap_or("");
        let marker = parts.next().unwrap_or("");
        let label = parts.next().unwrap_or(label_with_change.as_str());
        let change_str = if marker.is_empty() {
            change.to_string()
        } else {
            format!("{marker} {change}")
        };
        let before = by_label.get(label).map(|e| e.before).unwrap_or(0.0);
        println!(
            "| {} | {} | {} | {} |",
            escape_cell(label),
            format_number(before),
            format_number(*after),
            change_str,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_change_covers_all_branches() {
        assert_eq!(format_change(Some(25.0), 5.0), "▲ +25%");
        assert_eq!(format_change(Some(-10.0), -2.0), "▼ -10%");
        assert_eq!(format_change(Some(0.0), 0.0), "─ 0%");
        assert_eq!(format_change(None, 1500.0), "▲ +1.5k");
        assert_eq!(format_change(None, -3.0), "▼ -3");
        assert_eq!(format_change(None, 0.0), "─");
    }

    #[test]
    fn escape_cell_escapes_pipes() {
        assert_eq!(escape_cell("a|b"), "a\\|b");
        assert_eq!(escape_cell("plain"), "plain");
    }
}
