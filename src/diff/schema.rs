//! Schema validation and column resolution for diff mode.

use anyhow::{Result, bail};
use std::path::Path;

use crate::cli::Cli;
use crate::loader::LoadedData;

/// Validate that two datasets have compatible schemas (same column names, case-insensitive).
pub fn validate_schema(
    before: &LoadedData,
    after: &LoadedData,
    before_path: &Path,
    after_path: &Path,
) -> Result<()> {
    let norm_before: Vec<String> = before
        .headers
        .iter()
        .map(|h| h.trim().to_lowercase())
        .collect();
    let norm_after: Vec<String> = after
        .headers
        .iter()
        .map(|h| h.trim().to_lowercase())
        .collect();

    if norm_before.len() != norm_after.len() {
        bail!(
            "Schema mismatch: '{}' has {} columns [{}], '{}' has {} columns [{}]",
            before_path.display(),
            before.headers.len(),
            before.headers.join(", "),
            after_path.display(),
            after.headers.len(),
            after.headers.join(", "),
        );
    }

    // Check that all column names from before exist in after (case-insensitive)
    for col in &norm_before {
        if !norm_after.contains(col) {
            bail!(
                "Schema mismatch: column '{}' in '{}' not found in '{}'. \
                 Before columns: [{}], After columns: [{}]",
                col,
                before_path.display(),
                after_path.display(),
                before.headers.join(", "),
                after.headers.join(", "),
            );
        }
    }

    Ok(())
}

/// Resolve which column to use as X axis for diff comparison.
pub(super) fn resolve_x_column(
    cli: &Cli,
    data: &LoadedData,
    schema: &crate::infer::types::Schema,
) -> Result<String> {
    if let Some(ref x) = cli.x_col {
        let (col, _) = crate::cli::parse_column_spec(x);
        if !data.headers.iter().any(|h| h == col) {
            bail!(
                "X column '{}' not found. Available: {}",
                col,
                data.headers.join(", ")
            );
        }
        return Ok(col.to_string());
    }

    // Auto-detect: first categorical/temporal column
    use crate::infer::types::DataType;
    for col_meta in &schema.columns {
        if col_meta.data_type == DataType::Categorical || col_meta.data_type == DataType::Temporal {
            return Ok(col_meta.name.clone());
        }
    }

    // Fallback to first column
    data.headers
        .first()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("No columns available for X axis"))
}

/// Resolve which column to use as Y axis for diff comparison.
pub(super) fn resolve_y_column(
    cli: &Cli,
    data: &LoadedData,
    schema: &crate::infer::types::Schema,
    x_col: &str,
) -> Result<String> {
    if let Some(ref y) = cli.y_col {
        let (col, _) = crate::cli::parse_column_spec(y);
        if !data.headers.iter().any(|h| h == col) {
            bail!(
                "Y column '{}' not found. Available: {}",
                col,
                data.headers.join(", ")
            );
        }
        return Ok(col.to_string());
    }

    // Auto-detect: first quantitative column that is not x_col
    use crate::infer::types::DataType;
    for col_meta in &schema.columns {
        if col_meta.data_type == DataType::Quantitative && col_meta.name != x_col {
            return Ok(col_meta.name.clone());
        }
    }

    bail!(
        "No quantitative column found for Y axis (excluding X='{}'). Available: {}",
        x_col,
        data.headers.join(", ")
    )
}

/// Find column index by name (case-sensitive, with case-insensitive fallback).
pub(super) fn col_index(headers: &[String], name: &str) -> Option<usize> {
    headers.iter().position(|h| h == name).or_else(|| {
        // Fallback: case-insensitive match
        let lower = name.to_lowercase();
        headers.iter().position(|h| h.to_lowercase() == lower)
    })
}
