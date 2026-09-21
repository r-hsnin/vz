//! Schema validation and column resolution for diff mode.

use anyhow::{Result, bail};
use std::path::Path;

use crate::chart::Query;
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

/// Auto-detect X column: first categorical/temporal column,
/// falling back to the first header.
pub fn auto_x_column(schema: &crate::infer::types::Schema, headers: &[String]) -> Option<String> {
    use crate::infer::types::DataType;
    schema
        .columns
        .iter()
        .find(|c| c.data_type == DataType::Categorical || c.data_type == DataType::Temporal)
        .map(|c| c.name.clone())
        .or_else(|| headers.first().cloned())
}

/// Auto-detect Y column: first quantitative column that is not `x_col`.
pub fn auto_y_column(schema: &crate::infer::types::Schema, x_col: &str) -> Option<String> {
    use crate::infer::types::DataType;
    schema
        .columns
        .iter()
        .find(|c| c.data_type == DataType::Quantitative && c.name != x_col)
        .map(|c| c.name.clone())
}

/// Check whether a column is temporal (unknown columns read as non-temporal).
pub fn is_temporal_column(schema: &crate::infer::types::Schema, col: &str) -> bool {
    use crate::infer::types::DataType;
    schema
        .find_column(col)
        .map(|c| c.data_type == DataType::Temporal)
        .unwrap_or(false)
}

/// Resolve which column to use as X axis for diff comparison.
pub(crate) fn resolve_x_column(
    query: &Query,
    data: &LoadedData,
    schema: &crate::infer::types::Schema,
) -> Result<String> {
    if let Some(ref x) = query.x_col {
        let (col, _) = crate::cli::parse_column_spec(x);
        if !data.headers.iter().any(|h| h == col) {
            let suffix = crate::diagnostics::format_column_suffix(
                crate::diagnostics::suggest_column(&data.headers, col).as_deref(),
                col,
            );
            bail!(
                "X column '{}' not found. Available: {}{}",
                col,
                data.headers.join(", "),
                suffix
            );
        }
        return Ok(col.to_string());
    }

    auto_x_column(schema, &data.headers)
        .ok_or_else(|| anyhow::anyhow!("No columns available for X axis"))
}

/// Resolve which column to use as Y axis for diff comparison.
pub(crate) fn resolve_y_column(
    query: &Query,
    data: &LoadedData,
    schema: &crate::infer::types::Schema,
    x_col: &str,
) -> Result<String> {
    if let Some(ref y) = query.y_col {
        let (col, _) = crate::cli::parse_column_spec(y);
        if !data.headers.iter().any(|h| h == col) {
            let suffix = crate::diagnostics::format_column_suffix(
                crate::diagnostics::suggest_column(&data.headers, col).as_deref(),
                col,
            );
            bail!(
                "Y column '{}' not found. Available: {}{}",
                col,
                data.headers.join(", "),
                suffix
            );
        }
        return Ok(col.to_string());
    }

    auto_y_column(schema, x_col).ok_or_else(|| {
        anyhow::anyhow!(
            "No quantitative column found for Y axis (excluding X='{}'). Available: {}",
            x_col,
            data.headers.join(", ")
        )
    })
}

/// Find column index by name (case-sensitive, with case-insensitive fallback).
pub(super) fn col_index(headers: &[String], name: &str) -> Option<usize> {
    headers.iter().position(|h| h == name).or_else(|| {
        // Fallback: case-insensitive match
        let lower = name.to_lowercase();
        headers.iter().position(|h| h.to_lowercase() == lower)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infer::types::DataType;
    use crate::test_helpers::make_schema;

    fn headers(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn auto_x_prefers_first_categorical_or_temporal() {
        let schema = make_schema(&[
            ("date", DataType::Temporal),
            ("city", DataType::Categorical),
            ("region", DataType::Categorical),
        ]);
        assert_eq!(
            auto_x_column(&schema, &headers(&["date", "city", "region"])),
            Some("date".to_string())
        );
    }

    #[test]
    fn auto_x_skips_quantitative() {
        let schema = make_schema(&[
            ("revenue", DataType::Quantitative),
            ("city", DataType::Categorical),
        ]);
        assert_eq!(
            auto_x_column(&schema, &headers(&["revenue", "city"])),
            Some("city".to_string())
        );
    }

    #[test]
    fn auto_x_falls_back_to_first_header() {
        let schema = make_schema(&[
            ("revenue", DataType::Quantitative),
            ("profit", DataType::Quantitative),
        ]);
        assert_eq!(
            auto_x_column(&schema, &headers(&["revenue", "profit"])),
            Some("revenue".to_string())
        );
    }

    #[test]
    fn auto_x_returns_none_without_columns() {
        let schema = make_schema(&[]);
        assert_eq!(auto_x_column(&schema, &[]), None);
    }

    #[test]
    fn auto_y_skips_x_column() {
        let schema = make_schema(&[
            ("revenue", DataType::Quantitative),
            ("profit", DataType::Quantitative),
        ]);
        assert_eq!(
            auto_y_column(&schema, "revenue"),
            Some("profit".to_string())
        );
    }

    #[test]
    fn auto_y_returns_none_without_quantitative() {
        let schema = make_schema(&[
            ("date", DataType::Temporal),
            ("city", DataType::Categorical),
        ]);
        assert_eq!(auto_y_column(&schema, "city"), None);
    }

    #[test]
    fn is_temporal_column_detects_temporal() {
        let schema = make_schema(&[
            ("date", DataType::Temporal),
            ("city", DataType::Categorical),
        ]);
        assert!(is_temporal_column(&schema, "date"));
        assert!(!is_temporal_column(&schema, "city"));
        assert!(!is_temporal_column(&schema, "missing"));
    }

    #[test]
    fn resolve_x_column_suggests_close_match() {
        let schema = make_schema(&[
            ("date", DataType::Temporal),
            ("revenue", DataType::Quantitative),
        ]);
        let query = Query {
            x_col: Some("revnue".to_string()),
            ..Default::default()
        };
        let data = crate::loader::LoadedData {
            headers: headers(&["date", "revenue"]),
            rows: vec![],
        };
        let err = resolve_x_column(&query, &data, &schema).unwrap_err();
        let msg = format!("{:#}", err);
        assert!(msg.contains("Did you mean 'revenue'?"), "{msg}");
    }

    #[test]
    fn resolve_y_column_marks_case_sensitivity() {
        let schema = make_schema(&[
            ("date", DataType::Temporal),
            ("revenue", DataType::Quantitative),
        ]);
        let query = Query {
            y_col: Some("Revenue".to_string()),
            ..Default::default()
        };
        let data = crate::loader::LoadedData {
            headers: headers(&["date", "revenue"]),
            rows: vec![],
        };
        let err = resolve_y_column(&query, &data, &schema, "date").unwrap_err();
        let msg = format!("{:#}", err);
        assert!(msg.contains("case-sensitive"), "{msg}");
    }
}
