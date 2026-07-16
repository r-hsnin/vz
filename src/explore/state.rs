use crate::infer::types::{DataType, Schema};

/// Find initial axes based on schema (prefer temporal for x, quantitative for y).
pub(crate) fn initial_axes(schema: &Schema) -> (usize, usize) {
    let temporal_idx = schema
        .columns
        .iter()
        .position(|c| c.data_type == DataType::Temporal);
    let quant_idx = schema
        .columns
        .iter()
        .position(|c| c.data_type == DataType::Quantitative);

    let x = temporal_idx
        .or_else(|| {
            schema
                .columns
                .iter()
                .position(|c| c.data_type == DataType::Categorical)
        })
        .unwrap_or(0);
    let y = quant_idx.unwrap_or(1.min(schema.columns.len().saturating_sub(1)));

    (x, y)
}
