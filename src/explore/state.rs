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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::make_schema;

    #[test]
    fn initial_axes_prefers_temporal_x_and_quant_y() {
        let schema = make_schema(&[
            ("city", DataType::Categorical),
            ("date", DataType::Temporal),
            ("sales", DataType::Quantitative),
        ]);
        assert_eq!(initial_axes(&schema), (1, 2));
    }

    #[test]
    fn initial_axes_falls_back_to_categorical_x() {
        let schema = make_schema(&[
            ("sales", DataType::Quantitative),
            ("city", DataType::Categorical),
        ]);
        assert_eq!(initial_axes(&schema), (1, 0));
    }

    #[test]
    fn initial_axes_without_quant_points_y_at_second_column() {
        let schema = make_schema(&[
            ("date", DataType::Temporal),
            ("city", DataType::Categorical),
        ]);
        assert_eq!(initial_axes(&schema), (0, 1));
    }

    #[test]
    fn initial_axes_single_column_stays_at_zero() {
        let schema = make_schema(&[("only", DataType::Nominal)]);
        assert_eq!(initial_axes(&schema), (0, 0));
    }

    #[test]
    fn initial_axes_empty_schema_defaults_to_zero() {
        let schema = make_schema(&[]);
        assert_eq!(initial_axes(&schema), (0, 0));
    }
}
