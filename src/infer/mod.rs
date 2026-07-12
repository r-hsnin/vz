pub mod detector;
pub mod types;

pub use types::{ColumnMeta, DataType, Schema};

/// Infer schema from raw tabular data (headers + rows of string values).
pub fn infer_schema(headers: &[&str], rows: &[Vec<&str>]) -> Schema {
    let columns = headers
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let values: Vec<&str> = rows.iter().filter_map(|row| row.get(i).copied()).collect();
            let null_count = values.iter().filter(|v| v.trim().is_empty()).count();
            let data_type = detector::infer_column_type(&values);

            ColumnMeta {
                name: name.to_string(),
                data_type,
                null_count,
                sample_size: values.len(),
            }
        })
        .collect();

    Schema::new(columns)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_schema_basic() {
        let headers = vec!["date", "city", "revenue"];
        let rows = vec![
            vec!["2024-01-01", "Tokyo", "100"],
            vec!["2024-02-01", "Osaka", "200"],
            vec!["2024-03-01", "Tokyo", "150"],
            vec!["2024-04-01", "Nagoya", "300"],
        ];

        let schema = infer_schema(&headers, &rows);

        assert_eq!(schema.columns.len(), 3);
        assert_eq!(schema.columns[0].data_type, DataType::Temporal);
        assert_eq!(schema.columns[1].data_type, DataType::Categorical);
        assert_eq!(schema.columns[2].data_type, DataType::Quantitative);
    }

    #[test]
    fn test_infer_schema_find_column() {
        let headers = vec!["month", "sales"];
        let rows = vec![vec!["2024-01-01", "500"], vec!["2024-02-01", "600"]];

        let schema = infer_schema(&headers, &rows);
        let col = schema.find_column("sales").unwrap();
        assert_eq!(col.data_type, DataType::Quantitative);
    }

    #[test]
    fn test_infer_schema_columns_of_type() {
        let headers = vec!["date", "revenue", "profit"];
        let rows = vec![
            vec!["2024-01-01", "100", "50"],
            vec!["2024-02-01", "200", "80"],
        ];

        let schema = infer_schema(&headers, &rows);
        let quant_cols = schema.columns_of_type(DataType::Quantitative);
        assert_eq!(quant_cols.len(), 2);
    }
}
