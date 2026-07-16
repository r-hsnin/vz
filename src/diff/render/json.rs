//! JSON output for diff mode (both categorical and temporal).

use anyhow::Result;
use std::path::Path;

use crate::diff::{DiffResult, DiffTimeSeries};

/// Print categorical diff as JSON.
pub(super) fn print_diff_json(
    diff: &DiffResult,
    before_path: &Path,
    after_path: &Path,
) -> Result<()> {
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

/// Print temporal diff as JSON.
pub(super) fn print_diff_line_json(
    ts: &DiffTimeSeries,
    before_path: &Path,
    after_path: &Path,
) -> Result<()> {
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
