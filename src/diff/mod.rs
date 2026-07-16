//! Diff mode: compare two data files with matching schemas.
//!
//! Loads both files, validates schema compatibility, computes deltas,
//! and renders diff-aware visualizations.

mod compute;
pub mod render;
mod schema;
#[cfg(test)]
mod tests;

pub use compute::{compute_diff, compute_diff_temporal};
pub use schema::validate_schema;

use anyhow::Result;
use std::path::Path;

use crate::cli::Cli;
use crate::helpers::format_override;
use crate::loader;

/// Per-category diff entry for bar-style comparison.
#[derive(Debug, Clone, PartialEq)]
pub struct DiffEntry {
    pub label: String,
    pub before: f64,
    pub after: f64,
    pub delta: f64,
    pub pct_change: Option<f64>,
}

/// Result of computing a diff between two datasets.
#[derive(Debug, Clone)]
pub struct DiffResult {
    pub entries: Vec<DiffEntry>,
    pub x_column: String,
    pub y_column: String,
    pub before_rows: usize,
    pub after_rows: usize,
    pub overall_pct: Option<f64>,
}

/// Result of computing a temporal diff between two time-series datasets.
/// Holds ordered (x_index, y_value) pairs for before/after, aligned by date.
#[derive(Debug, Clone)]
pub struct DiffTimeSeries {
    /// Before series as (x_index, y_value) pairs, ordered by date.
    pub before: Vec<(f64, f64)>,
    /// After series as (x_index, y_value) pairs, ordered by date.
    pub after: Vec<(f64, f64)>,
    /// Original date labels (union of both files), sorted chronologically.
    pub x_labels: Vec<String>,
    pub x_column: String,
    pub y_column: String,
    pub before_rows: usize,
    pub after_rows: usize,
    /// Overall percentage change (sum of after / sum of before - 1) * 100.
    pub overall_pct: Option<f64>,
}

/// Run diff mode: load both files, validate schemas, compute and render diff.
pub fn run_diff(cli: &Cli, before_path: &Path, after_path: &Path) -> Result<()> {
    let before = loader::load_data_full(before_path, cli.no_header, format_override(cli))?;
    let after = loader::load_data_full(after_path, cli.no_header, format_override(cli))?;

    schema::validate_schema(&before, &after, before_path, after_path)?;

    let inferred = crate::pipeline::infer_from_data(&before);
    let x_col = schema::resolve_x_column(cli, &before, &inferred)?;
    let y_col = schema::resolve_y_column(cli, &before, &inferred, &x_col)?;

    // Check if X column is temporal → use line chart overlay
    let x_is_temporal = inferred
        .find_column(&x_col)
        .map(|c| c.data_type == crate::infer::types::DataType::Temporal)
        .unwrap_or(false);

    if x_is_temporal {
        let ts = compute::compute_diff_temporal(&before, &after, &x_col, &y_col)?;
        render::render_diff_line(cli, &ts, before_path, after_path)
    } else {
        let diff = compute::compute_diff(&before, &after, &x_col, &y_col)?;
        render::render_diff(cli, &diff, before_path, after_path)
    }
}
