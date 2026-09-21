/// Data types inferred from column values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataType {
    /// Date/time values (ISO 8601, common formats)
    Temporal,
    /// Numeric values (integers, floats)
    Quantitative,
    /// Low cardinality text (≤ threshold unique values)
    Categorical,
    /// High cardinality text (UUIDs, free text)
    Nominal,
}

impl std::fmt::Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataType::Temporal => write!(f, "Date/Time"),
            DataType::Quantitative => write!(f, "Numeric"),
            DataType::Categorical => write!(f, "Categorical"),
            DataType::Nominal => write!(f, "Text"),
        }
    }
}

/// Metadata about a single column after inference.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnMeta {
    pub name: String,
    pub data_type: DataType,
    pub null_count: usize,
    pub sample_size: usize,
}

/// A complete schema for a dataset.
#[derive(Debug, Clone, PartialEq)]
pub struct Schema {
    pub columns: Vec<ColumnMeta>,
}

impl Schema {
    pub fn new(columns: Vec<ColumnMeta>) -> Self {
        Self { columns }
    }

    /// Find a column by name.
    pub fn find_column(&self, name: &str) -> Option<&ColumnMeta> {
        self.columns.iter().find(|c| c.name == name)
    }

    /// Get all columns of a specific type.
    pub fn columns_of_type(&self, dt: DataType) -> Vec<&ColumnMeta> {
        self.columns.iter().filter(|c| c.data_type == dt).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(name: &str, data_type: DataType) -> ColumnMeta {
        ColumnMeta {
            name: name.to_string(),
            data_type,
            null_count: 0,
            sample_size: 1,
        }
    }

    #[test]
    fn data_type_display_labels() {
        let cases = [
            (DataType::Temporal, "Date/Time"),
            (DataType::Quantitative, "Numeric"),
            (DataType::Categorical, "Categorical"),
            (DataType::Nominal, "Text"),
        ];
        for (dt, expected) in cases {
            assert_eq!(dt.to_string(), expected);
        }
    }

    #[test]
    fn schema_find_column_hit_and_miss() {
        let schema = Schema::new(vec![meta("a", DataType::Quantitative)]);
        assert_eq!(
            schema.find_column("a").unwrap().data_type,
            DataType::Quantitative
        );
        assert!(schema.find_column("missing").is_none());
    }

    #[test]
    fn schema_columns_of_type_filters() {
        let schema = Schema::new(vec![
            meta("d", DataType::Temporal),
            meta("q1", DataType::Quantitative),
            meta("q2", DataType::Quantitative),
        ]);
        let quant: Vec<&str> = schema
            .columns_of_type(DataType::Quantitative)
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert_eq!(quant, vec!["q1", "q2"]);
        assert!(schema.columns_of_type(DataType::Nominal).is_empty());
    }
}
