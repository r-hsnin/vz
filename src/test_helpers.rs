//! Shared fixtures for unit tests (test builds only).
//!
//! Centralizes the `make_schema` / `make_recommendation` helpers that were
//! previously copy-pasted across test modules.

use crate::chart::selector::{ChartRecommendation, ChartType};
use crate::infer::types::{ColumnMeta, DataType, Schema};

/// Build a [`Schema`] from `(name, type)` pairs for tests.
pub fn make_schema(cols: &[(&str, DataType)]) -> Schema {
    Schema::new(
        cols.iter()
            .map(|(name, dt)| ColumnMeta {
                name: name.to_string(),
                data_type: *dt,
                null_count: 0,
                sample_size: 10,
            })
            .collect(),
    )
}

/// Build a [`ChartRecommendation`] for tests.
pub fn make_recommendation(
    chart_type: ChartType,
    x: &str,
    y: Option<&str>,
    color: Option<&str>,
) -> ChartRecommendation {
    ChartRecommendation {
        chart_type,
        x_column: x.to_string(),
        y_column: y.map(|s| s.to_string()),
        color_column: color.map(|s| s.to_string()),
    }
}
