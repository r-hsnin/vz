//! Diff computation: categorical and temporal diff algorithms.

use anyhow::Result;

use crate::loader::LoadedData;

use super::schema::col_index;
use super::{DiffEntry, DiffResult, DiffTimeSeries};

/// Compute a temporal diff between two time-series datasets.
/// Aligns both datasets on a sorted union of X (date) labels, aggregating duplicates by sum.
pub fn compute_diff_temporal(
    before: &LoadedData,
    after: &LoadedData,
    x_col: &str,
    y_col: &str,
) -> Result<DiffTimeSeries> {
    let before_x_idx = col_index(&before.headers, x_col)
        .ok_or_else(|| anyhow::anyhow!("Column '{}' not found in before file", x_col))?;
    let before_y_idx = col_index(&before.headers, y_col)
        .ok_or_else(|| anyhow::anyhow!("Column '{}' not found in before file", y_col))?;
    let after_x_idx = col_index(&after.headers, x_col)
        .ok_or_else(|| anyhow::anyhow!("Column '{}' not found in after file", x_col))?;
    let after_y_idx = col_index(&after.headers, y_col)
        .ok_or_else(|| anyhow::anyhow!("Column '{}' not found in after file", y_col))?;

    // Aggregate by date (sum duplicates)
    let before_agg = aggregate_by_category(&before.rows, before_x_idx, before_y_idx);
    let after_agg = aggregate_by_category(&after.rows, after_x_idx, after_y_idx);

    // Build sorted union of all date labels (string sort works for ISO dates)
    let mut all_labels: Vec<String> = Vec::new();
    for (label, _) in &before_agg {
        if !all_labels.contains(label) {
            all_labels.push(label.clone());
        }
    }
    for (label, _) in &after_agg {
        if !all_labels.contains(label) {
            all_labels.push(label.clone());
        }
    }
    all_labels.sort();

    // Build series: each point at (x_index, y_value)
    let mut before_series = Vec::new();
    let mut after_series = Vec::new();
    let mut total_before = 0.0_f64;
    let mut total_after = 0.0_f64;

    for (i, label) in all_labels.iter().enumerate() {
        let x = i as f64;
        if let Some((_, v)) = before_agg.iter().find(|(l, _)| l == label) {
            before_series.push((x, *v));
            total_before += *v;
        }
        if let Some((_, v)) = after_agg.iter().find(|(l, _)| l == label) {
            after_series.push((x, *v));
            total_after += *v;
        }
    }

    let overall_pct = if total_before.abs() > f64::EPSILON {
        Some((total_after - total_before) / total_before * 100.0)
    } else {
        None
    };

    Ok(DiffTimeSeries {
        before: before_series,
        after: after_series,
        x_labels: all_labels,
        x_column: x_col.to_string(),
        y_column: y_col.to_string(),
        before_rows: before.rows.len(),
        after_rows: after.rows.len(),
        overall_pct,
    })
}

/// Compute the diff between two datasets on the given X/Y columns.
pub fn compute_diff(
    before: &LoadedData,
    after: &LoadedData,
    x_col: &str,
    y_col: &str,
) -> Result<DiffResult> {
    let before_x_idx = col_index(&before.headers, x_col)
        .ok_or_else(|| anyhow::anyhow!("Column '{}' not found in before file", x_col))?;
    let before_y_idx = col_index(&before.headers, y_col)
        .ok_or_else(|| anyhow::anyhow!("Column '{}' not found in before file", y_col))?;
    let after_x_idx = col_index(&after.headers, x_col)
        .ok_or_else(|| anyhow::anyhow!("Column '{}' not found in after file", x_col))?;
    let after_y_idx = col_index(&after.headers, y_col)
        .ok_or_else(|| anyhow::anyhow!("Column '{}' not found in after file", y_col))?;

    // Aggregate by X category (sum values for duplicate categories)
    let before_agg = aggregate_by_category(&before.rows, before_x_idx, before_y_idx);
    let after_agg = aggregate_by_category(&after.rows, after_x_idx, after_y_idx);

    // Build diff entries: all categories from both sides
    let mut all_labels: Vec<String> = Vec::new();
    for (label, _) in &before_agg {
        if !all_labels.contains(label) {
            all_labels.push(label.clone());
        }
    }
    for (label, _) in &after_agg {
        if !all_labels.contains(label) {
            all_labels.push(label.clone());
        }
    }

    let mut entries = Vec::new();
    let mut total_before = 0.0_f64;
    let mut total_after = 0.0_f64;

    for label in &all_labels {
        let bv = before_agg
            .iter()
            .find(|(l, _)| l == label)
            .map(|(_, v)| *v)
            .unwrap_or(0.0);
        let av = after_agg
            .iter()
            .find(|(l, _)| l == label)
            .map(|(_, v)| *v)
            .unwrap_or(0.0);
        let delta = av - bv;
        let pct = if bv.abs() > f64::EPSILON {
            Some(delta / bv * 100.0)
        } else if av.abs() > f64::EPSILON {
            None // from zero → show as new (no percentage)
        } else {
            Some(0.0) // both zero
        };

        total_before += bv;
        total_after += av;

        entries.push(DiffEntry {
            label: label.clone(),
            before: bv,
            after: av,
            delta,
            pct_change: pct,
        });
    }

    let overall_pct = if total_before.abs() > f64::EPSILON {
        Some((total_after - total_before) / total_before * 100.0)
    } else {
        None
    };

    Ok(DiffResult {
        entries,
        x_column: x_col.to_string(),
        y_column: y_col.to_string(),
        before_rows: before.rows.len(),
        after_rows: after.rows.len(),
        overall_pct,
    })
}

/// Aggregate values by category (sum duplicate keys).
fn aggregate_by_category(rows: &[Vec<String>], x_idx: usize, y_idx: usize) -> Vec<(String, f64)> {
    let mut map: Vec<(String, f64)> = Vec::new();
    for row in rows {
        let label = row.get(x_idx).map(|s| s.as_str()).unwrap_or("");
        let value = row
            .get(y_idx)
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        if let Some(entry) = map.iter_mut().find(|(l, _)| l == label) {
            entry.1 += value;
        } else {
            map.push((label.to_string(), value));
        }
    }
    map
}
