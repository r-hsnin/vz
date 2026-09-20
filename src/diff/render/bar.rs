//! Bar chart rendering for categorical diff output.

use std::path::Path;

use crate::cli::DiffParams;
use crate::diff::DiffResult;
use crate::render::format_number;
use crate::util::path_label;

/// Default chart width for diff bar rendering when terminal width is unavailable.
const DEFAULT_DIFF_BAR_WIDTH: usize = 60;

/// Print the diff summary line: `Diff │ x=col │ y=col │ before vs after │ Δ net +N% │ N entries`
pub(super) fn print_diff_summary(diff: &DiffResult, before_path: &Path, after_path: &Path) {
    let before_name = path_label(before_path);
    let after_name = path_label(after_path);

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

    for line in crate::insights::diff_insights(
        &diff
            .entries
            .iter()
            .map(|e| (e.label.clone(), e.before, e.after, e.pct_change))
            .collect::<Vec<_>>(),
    ) {
        eprintln!("💡 {line}");
    }
}

/// Print a diff-aware bar chart with ▲/▼ direction markers.
pub(super) fn print_diff_bar(params: &DiffParams, diff: &DiffResult) {
    // Canonical sort/limit + direction annotation (labels are already
    // "label ▲ +20%"); this edge keeps only width/fitting + the
    // `before → after` text layout.
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
    if data.labels.is_empty() {
        return;
    }
    // Look up before/after by label (labels are unique per diff entry).
    let by_label: std::collections::HashMap<&str, &crate::diff::DiffEntry> =
        diff.entries.iter().map(|e| (e.label.as_str(), e)).collect();

    // Find max absolute after value for bar scaling
    let max_abs = data.values.iter().map(|v| v.abs()).fold(0.0_f64, f64::max);

    // Width math stays on plain labels (annotation is appended after, so
    // the bar geometry matches the pre-canonical layout exactly).
    let plain_labels: Vec<&str> = data
        .labels
        .iter()
        .map(|label_with_change| {
            let mut parts = label_with_change.rsplitn(3, ' ');
            let _ = parts.next();
            let _ = parts.next();
            parts.next().unwrap_or(label_with_change.as_str())
        })
        .collect();
    let label_width = plain_labels.iter().map(|l| l.len()).max().unwrap_or(8);
    let bar_width: usize = params
        .width
        .map(|w| w as usize)
        .unwrap_or(DEFAULT_DIFF_BAR_WIDTH)
        .saturating_sub(label_width + 40);
    let bar_width = bar_width.max(10);

    for (label_with_change, after) in data.labels.iter().zip(data.values.iter()) {
        // Canonical labels are "label ▲ +20%": split off the last two
        // tokens (marker + change) to recover the plain label.
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

        let bar_len = if max_abs > 0.0 {
            ((after.abs() / max_abs) * bar_width as f64).round() as usize
        } else {
            0
        };
        let bar = "█".repeat(bar_len);

        println!(
            "  {:width$}  {}  {} → {}  {}",
            label,
            bar,
            format_number(before),
            format_number(*after),
            change_str,
            width = label_width,
        );
    }
}
