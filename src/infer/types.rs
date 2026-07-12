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
