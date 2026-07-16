//! Bar chart rendering for categorical diff output.

use std::path::Path;

use crate::cli::Cli;
use crate::diff::DiffResult;
use crate::render::format_number;

use super::apply_sort_and_limit;

/// Print the diff summary line: `Diff │ x=col │ y=col │ before vs after │ Δ net +N% │ N entries`
pub(super) fn print_diff_summary(diff: &DiffResult, before_path: &Path, after_path: &Path) {
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
pub(super) fn print_diff_bar(cli: &Cli, diff: &DiffResult) {
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
