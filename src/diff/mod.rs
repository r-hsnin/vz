//! Diff mode: compare two data files with matching schemas.
//!
//! Loads both files, validates schema compatibility, computes deltas,
//! and renders diff-aware visualizations.

pub mod render;

use anyhow::{Result, bail};
use std::path::Path;

use crate::cli::Cli;
use crate::helpers::format_override;
use crate::loader::{self, LoadedData};

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

/// Run diff mode: load both files, validate schemas, compute and render diff.
pub fn run_diff(cli: &Cli, before_path: &Path, after_path: &Path) -> Result<()> {
    let before = loader::load_data_full(before_path, cli.no_header, format_override(cli))?;
    let after = loader::load_data_full(after_path, cli.no_header, format_override(cli))?;

    validate_schema(&before, &after, before_path, after_path)?;

    let schema = crate::pipeline::infer_from_data(&before);
    let x_col = resolve_x_column(cli, &before, &schema)?;
    let y_col = resolve_y_column(cli, &before, &schema, &x_col)?;

    let diff = compute_diff(&before, &after, &x_col, &y_col)?;

    render::render_diff(cli, &diff, before_path, after_path)
}

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
fn resolve_x_column(
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
fn resolve_y_column(
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

/// Find column index by name (case-sensitive).
fn col_index(headers: &[String], name: &str) -> Option<usize> {
    headers.iter().position(|h| h == name).or_else(|| {
        // Fallback: case-insensitive match
        let lower = name.to_lowercase();
        headers.iter().position(|h| h.to_lowercase() == lower)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::path::PathBuf;

    fn make_data(headers: &[&str], rows: &[&[&str]]) -> LoadedData {
        LoadedData {
            headers: headers.iter().map(|s| s.to_string()).collect(),
            rows: rows
                .iter()
                .map(|r| r.iter().map(|s| s.to_string()).collect())
                .collect(),
        }
    }

    #[test]
    fn test_validate_schema_same_columns() {
        let a = make_data(&["city", "revenue"], &[&["Tokyo", "100"]]);
        let b = make_data(&["city", "revenue"], &[&["Tokyo", "200"]]);
        let r = validate_schema(&a, &b, Path::new("a.csv"), Path::new("b.csv"));
        assert!(r.is_ok());
    }

    #[test]
    fn test_validate_schema_case_insensitive() {
        let a = make_data(&["City", "Revenue"], &[&["Tokyo", "100"]]);
        let b = make_data(&["city", "revenue"], &[&["Tokyo", "200"]]);
        let r = validate_schema(&a, &b, Path::new("a.csv"), Path::new("b.csv"));
        assert!(r.is_ok());
    }

    #[test]
    fn test_validate_schema_different_column_count() {
        let a = make_data(&["city", "revenue"], &[&["Tokyo", "100"]]);
        let b = make_data(&["city", "revenue", "profit"], &[&["Tokyo", "200", "50"]]);
        let r = validate_schema(&a, &b, Path::new("a.csv"), Path::new("b.csv"));
        assert!(r.is_err());
        assert!(format!("{}", r.unwrap_err()).contains("Schema mismatch"));
    }

    #[test]
    fn test_validate_schema_different_column_names() {
        let a = make_data(&["city", "revenue"], &[&["Tokyo", "100"]]);
        let b = make_data(&["product", "cost"], &[&["Widget", "50"]]);
        let r = validate_schema(&a, &b, Path::new("a.csv"), Path::new("b.csv"));
        assert!(r.is_err());
    }

    #[test]
    fn test_compute_diff_basic_increase() {
        let a = make_data(
            &["city", "revenue"],
            &[&["Tokyo", "1000"], &["Osaka", "1500"]],
        );
        let b = make_data(
            &["city", "revenue"],
            &[&["Tokyo", "1200"], &["Osaka", "1350"]],
        );
        let diff = compute_diff(&a, &b, "city", "revenue").unwrap();
        assert_eq!(diff.entries.len(), 2);

        let tokyo = &diff.entries[0];
        assert_eq!(tokyo.label, "Tokyo");
        assert_eq!(tokyo.before, 1000.0);
        assert_eq!(tokyo.after, 1200.0);
        assert_eq!(tokyo.delta, 200.0);
        assert!((tokyo.pct_change.unwrap() - 20.0).abs() < 0.01);

        let osaka = &diff.entries[1];
        assert_eq!(osaka.label, "Osaka");
        assert_eq!(osaka.before, 1500.0);
        assert_eq!(osaka.after, 1350.0);
        assert_eq!(osaka.delta, -150.0);
        assert!((osaka.pct_change.unwrap() - (-10.0)).abs() < 0.01);
    }

    #[test]
    fn test_compute_diff_no_change() {
        let a = make_data(&["city", "revenue"], &[&["Tokyo", "500"]]);
        let b = make_data(&["city", "revenue"], &[&["Tokyo", "500"]]);
        let diff = compute_diff(&a, &b, "city", "revenue").unwrap();
        assert_eq!(diff.entries[0].delta, 0.0);
        assert_eq!(diff.entries[0].pct_change, Some(0.0));
    }

    #[test]
    fn test_compute_diff_from_zero() {
        let a = make_data(&["city", "revenue"], &[&["Tokyo", "0"]]);
        let b = make_data(&["city", "revenue"], &[&["Tokyo", "500"]]);
        let diff = compute_diff(&a, &b, "city", "revenue").unwrap();
        // From zero: no percentage (shown as "new")
        assert_eq!(diff.entries[0].delta, 500.0);
        assert_eq!(diff.entries[0].pct_change, None);
    }

    #[test]
    fn test_compute_diff_new_category_in_after() {
        let a = make_data(&["city", "revenue"], &[&["Tokyo", "1000"]]);
        let b = make_data(
            &["city", "revenue"],
            &[&["Tokyo", "1200"], &["Osaka", "800"]],
        );
        let diff = compute_diff(&a, &b, "city", "revenue").unwrap();
        assert_eq!(diff.entries.len(), 2);
        let osaka = &diff.entries[1];
        assert_eq!(osaka.label, "Osaka");
        assert_eq!(osaka.before, 0.0);
        assert_eq!(osaka.after, 800.0);
        assert_eq!(osaka.delta, 800.0);
        assert_eq!(osaka.pct_change, None); // from zero
    }

    #[test]
    fn test_compute_diff_removed_category() {
        let a = make_data(
            &["city", "revenue"],
            &[&["Tokyo", "1000"], &["Osaka", "800"]],
        );
        let b = make_data(&["city", "revenue"], &[&["Tokyo", "1200"]]);
        let diff = compute_diff(&a, &b, "city", "revenue").unwrap();
        assert_eq!(diff.entries.len(), 2);
        let osaka = &diff.entries[1];
        assert_eq!(osaka.label, "Osaka");
        assert_eq!(osaka.before, 800.0);
        assert_eq!(osaka.after, 0.0);
        assert_eq!(osaka.delta, -800.0);
        assert!((osaka.pct_change.unwrap() - (-100.0)).abs() < 0.01);
    }

    #[test]
    fn test_compute_diff_aggregates_duplicates() {
        let a = make_data(
            &["city", "revenue"],
            &[&["Tokyo", "500"], &["Tokyo", "500"], &["Osaka", "300"]],
        );
        let b = make_data(
            &["city", "revenue"],
            &[&["Tokyo", "600"], &["Tokyo", "600"], &["Osaka", "400"]],
        );
        let diff = compute_diff(&a, &b, "city", "revenue").unwrap();
        let tokyo = &diff.entries[0];
        assert_eq!(tokyo.before, 1000.0); // 500+500
        assert_eq!(tokyo.after, 1200.0); // 600+600
    }

    #[test]
    fn test_compute_diff_overall_pct() {
        let a = make_data(
            &["city", "revenue"],
            &[&["Tokyo", "1000"], &["Osaka", "1500"]],
        );
        let b = make_data(
            &["city", "revenue"],
            &[&["Tokyo", "1200"], &["Osaka", "1350"]],
        );
        let diff = compute_diff(&a, &b, "city", "revenue").unwrap();
        // total before: 2500, total after: 2550, change: +50/2500 = +2%
        assert!((diff.overall_pct.unwrap() - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_col_index_case_insensitive_fallback() {
        let headers: Vec<String> = vec!["City".to_string(), "Revenue".to_string()];
        assert_eq!(col_index(&headers, "city"), Some(0));
        assert_eq!(col_index(&headers, "Revenue"), Some(1));
        assert_eq!(col_index(&headers, "missing"), None);
    }

    #[test]
    fn test_diff_pair_two_positional() {
        let cli = Cli::try_parse_from(["vz", "a.csv", "b.csv"]).unwrap();
        let pair = cli.diff_pair();
        assert_eq!(pair, Some((PathBuf::from("a.csv"), PathBuf::from("b.csv"))));
    }

    #[test]
    fn test_diff_pair_with_flag() {
        let cli = Cli::try_parse_from(["vz", "a.csv", "--diff", "b.csv"]).unwrap();
        let pair = cli.diff_pair();
        assert_eq!(pair, Some((PathBuf::from("a.csv"), PathBuf::from("b.csv"))));
    }

    #[test]
    fn test_diff_pair_single_file_no_diff() {
        let cli = Cli::try_parse_from(["vz", "a.csv"]).unwrap();
        assert_eq!(cli.diff_pair(), None);
    }
}
