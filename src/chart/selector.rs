use crate::infer::types::{ColumnMeta, DataType, Schema};

/// Supported chart types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChartType {
    Line,
    Bar,
    Scatter,
    Histogram,
    Heatmap,
}

impl std::fmt::Display for ChartType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChartType::Line => write!(f, "Line"),
            ChartType::Bar => write!(f, "Bar"),
            ChartType::Scatter => write!(f, "Scatter"),
            ChartType::Histogram => write!(f, "Histogram"),
            ChartType::Heatmap => write!(f, "Heatmap"),
        }
    }
}

/// Chart recommendation with axis assignments.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartRecommendation {
    pub chart_type: ChartType,
    pub x_column: String,
    pub y_column: Option<String>,
    pub color_column: Option<String>,
}

/// Select the best chart type based on schema and optional user hints.
pub fn select_chart(
    schema: &Schema,
    x_hint: Option<&str>,
    y_hint: Option<&str>,
) -> Result<ChartRecommendation, String> {
    // If user specified both axes, use them
    if let (Some(x_name), Some(y_name)) = (x_hint, y_hint) {
        let x_col = validate_column(schema, x_name)?;
        let y_col = validate_column(schema, y_name)?;
        let chart_type = chart_type_for_pair(x_col.data_type, y_col.data_type);
        return Ok(ChartRecommendation {
            chart_type,
            x_column: x_name.to_string(),
            y_column: Some(y_name.to_string()),
            color_column: find_color_column(schema, x_name, y_name),
        });
    }

    // Only Y specified: honor it, auto-select best X
    if let Some(y_name) = y_hint {
        return select_with_y_hint(schema, y_name);
    }

    // Only X specified: honor it, auto-select best Y
    if let Some(x_name) = x_hint {
        return select_with_x_hint(schema, x_name);
    }

    // No hints: auto-detect from schema
    auto_select(schema)
}

/// Validate that a column exists in the schema, returning a descriptive error if not.
fn validate_column<'a>(schema: &'a Schema, name: &str) -> Result<&'a ColumnMeta, String> {
    schema.find_column(name).ok_or_else(|| {
        format!(
            "Column '{}' not found. Available columns: {}",
            name,
            schema
                .columns
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

/// Select chart with Y column fixed by user hint, auto-picking the best X.
fn select_with_y_hint(schema: &Schema, y_name: &str) -> Result<ChartRecommendation, String> {
    let y_col = validate_column(schema, y_name)?;
    let temporal_cols = schema.columns_of_type(DataType::Temporal);
    let cat_cols = schema.columns_of_type(DataType::Categorical);
    let quant_cols: Vec<&ColumnMeta> = schema
        .columns_of_type(DataType::Quantitative)
        .into_iter()
        .filter(|c| c.name != y_name)
        .collect();

    // Best X: temporal > categorical > quantitative (excluding Y itself)
    if let Some(x) = temporal_cols.first() {
        let chart_type = chart_type_for_pair(x.data_type, y_col.data_type);
        return Ok(ChartRecommendation {
            chart_type,
            x_column: x.name.clone(),
            y_column: Some(y_name.to_string()),
            color_column: find_color_column(schema, &x.name, y_name),
        });
    }
    if let Some(x) = cat_cols.first() {
        let chart_type = chart_type_for_pair(x.data_type, y_col.data_type);
        return Ok(ChartRecommendation {
            chart_type,
            x_column: x.name.clone(),
            y_column: Some(y_name.to_string()),
            color_column: find_color_column(schema, &x.name, y_name),
        });
    }
    if let Some(x) = quant_cols.first() {
        let chart_type = chart_type_for_pair(x.data_type, y_col.data_type);
        return Ok(ChartRecommendation {
            chart_type,
            x_column: x.name.clone(),
            y_column: Some(y_name.to_string()),
            color_column: None,
        });
    }

    // Y is the only column — treat as histogram
    Ok(ChartRecommendation {
        chart_type: ChartType::Histogram,
        x_column: y_name.to_string(),
        y_column: None,
        color_column: None,
    })
}

/// Select chart with X column fixed by user hint, auto-picking the best Y.
fn select_with_x_hint(schema: &Schema, x_name: &str) -> Result<ChartRecommendation, String> {
    let x_col = validate_column(schema, x_name)?;
    let quant_cols: Vec<&ColumnMeta> = schema
        .columns_of_type(DataType::Quantitative)
        .into_iter()
        .filter(|c| c.name != x_name)
        .collect();

    // Best Y: first quantitative column not equal to X
    if let Some(y) = quant_cols.first() {
        let chart_type = chart_type_for_pair(x_col.data_type, y.data_type);
        return Ok(ChartRecommendation {
            chart_type,
            x_column: x_name.to_string(),
            y_column: Some(y.name.clone()),
            color_column: find_color_column(schema, x_name, &y.name),
        });
    }

    // No quantitative Y available — histogram of X if quantitative
    if x_col.data_type == DataType::Quantitative {
        return Ok(ChartRecommendation {
            chart_type: ChartType::Histogram,
            x_column: x_name.to_string(),
            y_column: None,
            color_column: None,
        });
    }

    Err(format!(
        "Cannot find a suitable Y axis column to pair with '{}'. \
         Hint: add a numeric column or specify -y explicitly.",
        x_name
    ))
}

/// Determine chart type from a pair of data types.
fn chart_type_for_pair(x_type: DataType, y_type: DataType) -> ChartType {
    match (x_type, y_type) {
        (DataType::Temporal, DataType::Quantitative) => ChartType::Line,
        (DataType::Categorical, DataType::Quantitative) => ChartType::Bar,
        (DataType::Quantitative, DataType::Quantitative) => ChartType::Scatter,
        (DataType::Categorical, DataType::Categorical) => ChartType::Heatmap,
        (DataType::Quantitative, DataType::Temporal) => ChartType::Line, // flip semantics
        (DataType::Quantitative, DataType::Categorical) => ChartType::Bar, // flip
        _ => ChartType::Bar,                                             // sensible fallback
    }
}

/// Auto-select chart based on schema column types.
fn auto_select(schema: &Schema) -> Result<ChartRecommendation, String> {
    let temporal_cols = schema.columns_of_type(DataType::Temporal);
    let quant_cols = schema.columns_of_type(DataType::Quantitative);
    let cat_cols = schema.columns_of_type(DataType::Categorical);

    // Priority 1: Temporal × Quantitative → Line
    if let (Some(x), Some(y)) = (temporal_cols.first(), quant_cols.first()) {
        return Ok(ChartRecommendation {
            chart_type: ChartType::Line,
            x_column: x.name.clone(),
            y_column: Some(y.name.clone()),
            color_column: cat_cols.first().map(|c| c.name.clone()),
        });
    }

    // Priority 2: Categorical × Quantitative → Bar
    if let (Some(x), Some(y)) = (cat_cols.first(), quant_cols.first()) {
        return Ok(ChartRecommendation {
            chart_type: ChartType::Bar,
            x_column: x.name.clone(),
            y_column: Some(y.name.clone()),
            color_column: None,
        });
    }

    // Priority 3: Quantitative × Quantitative → Scatter
    if quant_cols.len() >= 2 {
        return Ok(ChartRecommendation {
            chart_type: ChartType::Scatter,
            x_column: quant_cols[0].name.clone(),
            y_column: Some(quant_cols[1].name.clone()),
            color_column: cat_cols.first().map(|c| c.name.clone()),
        });
    }

    // Priority 4: Single Quantitative → Histogram
    if quant_cols.len() == 1 {
        return Ok(ChartRecommendation {
            chart_type: ChartType::Histogram,
            x_column: quant_cols[0].name.clone(),
            y_column: None,
            color_column: None,
        });
    }

    // Priority 5: Two categorical columns → Heatmap (count matrix)
    if cat_cols.len() >= 2 {
        return Ok(ChartRecommendation {
            chart_type: ChartType::Heatmap,
            x_column: cat_cols[0].name.clone(),
            y_column: Some(cat_cols[1].name.clone()),
            color_column: None,
        });
    }

    Err(no_chart_error(schema))
}

/// Build a descriptive error when no chart type can be inferred.
fn no_chart_error(schema: &Schema) -> String {
    if schema.columns.is_empty() {
        return "No columns detected in data. Check that your file is not empty.".to_string();
    }
    let type_summary: Vec<String> = schema
        .columns
        .iter()
        .map(|c| format!("{}={}", c.name, c.data_type))
        .collect();
    format!(
        "Could not determine chart type. Detected columns: [{}]. \
         Hint: specify axes with -x and -y, or ensure data has numeric/date columns.",
        type_summary.join(", ")
    )
}

/// Find a suitable color column (categorical, not already used as x or y).
fn find_color_column(schema: &Schema, x_name: &str, y_name: &str) -> Option<String> {
    schema
        .columns_of_type(DataType::Categorical)
        .into_iter()
        .find(|c| c.name != x_name && c.name != y_name)
        .map(|c| c.name.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infer::types::ColumnMeta;

    fn make_schema(cols: Vec<(&str, DataType)>) -> Schema {
        Schema::new(
            cols.into_iter()
                .map(|(name, dt)| ColumnMeta {
                    name: name.to_string(),
                    data_type: dt,
                    null_count: 0,
                    sample_size: 100,
                })
                .collect(),
        )
    }

    // --- Auto selection tests ---

    #[test]
    fn test_temporal_quantitative_gives_line() {
        let schema = make_schema(vec![
            ("date", DataType::Temporal),
            ("revenue", DataType::Quantitative),
        ]);
        let rec = select_chart(&schema, None, None).unwrap();
        assert_eq!(rec.chart_type, ChartType::Line);
        assert_eq!(rec.x_column, "date");
        assert_eq!(rec.y_column.as_deref(), Some("revenue"));
    }

    #[test]
    fn test_categorical_quantitative_gives_bar() {
        let schema = make_schema(vec![
            ("city", DataType::Categorical),
            ("sales", DataType::Quantitative),
        ]);
        let rec = select_chart(&schema, None, None).unwrap();
        assert_eq!(rec.chart_type, ChartType::Bar);
        assert_eq!(rec.x_column, "city");
        assert_eq!(rec.y_column.as_deref(), Some("sales"));
    }

    #[test]
    fn test_two_quantitative_gives_scatter() {
        let schema = make_schema(vec![
            ("height", DataType::Quantitative),
            ("weight", DataType::Quantitative),
        ]);
        let rec = select_chart(&schema, None, None).unwrap();
        assert_eq!(rec.chart_type, ChartType::Scatter);
        assert_eq!(rec.x_column, "height");
        assert_eq!(rec.y_column.as_deref(), Some("weight"));
    }

    #[test]
    fn test_single_quantitative_gives_histogram() {
        let schema = make_schema(vec![("age", DataType::Quantitative)]);
        let rec = select_chart(&schema, None, None).unwrap();
        assert_eq!(rec.chart_type, ChartType::Histogram);
        assert_eq!(rec.x_column, "age");
        assert_eq!(rec.y_column, None);
    }

    #[test]
    fn test_two_categorical_gives_bar() {
        let schema = make_schema(vec![
            ("department", DataType::Categorical),
            ("status", DataType::Categorical),
        ]);
        let rec = select_chart(&schema, None, None).unwrap();
        assert_eq!(rec.chart_type, ChartType::Heatmap);
        assert_eq!(rec.x_column, "department");
        assert_eq!(rec.y_column.as_deref(), Some("status"));
    }

    #[test]
    fn test_temporal_priority_over_categorical() {
        // When both temporal and categorical exist with quantitative,
        // temporal should win (line chart)
        let schema = make_schema(vec![
            ("date", DataType::Temporal),
            ("city", DataType::Categorical),
            ("revenue", DataType::Quantitative),
        ]);
        let rec = select_chart(&schema, None, None).unwrap();
        assert_eq!(rec.chart_type, ChartType::Line);
        assert_eq!(rec.x_column, "date");
        assert_eq!(rec.color_column.as_deref(), Some("city"));
    }

    // --- User hint tests ---

    #[test]
    fn test_user_specified_axes() {
        let schema = make_schema(vec![
            ("date", DataType::Temporal),
            ("city", DataType::Categorical),
            ("revenue", DataType::Quantitative),
        ]);
        let rec = select_chart(&schema, Some("city"), Some("revenue")).unwrap();
        assert_eq!(rec.chart_type, ChartType::Bar);
        assert_eq!(rec.x_column, "city");
        assert_eq!(rec.y_column.as_deref(), Some("revenue"));
    }

    #[test]
    fn test_user_specified_nonexistent_column() {
        let schema = make_schema(vec![("date", DataType::Temporal)]);
        let rec = select_chart(&schema, Some("nonexistent"), Some("also_bad"));
        assert!(rec.is_err());
        let err = rec.unwrap_err();
        assert!(err.contains("nonexistent"));
        assert!(err.contains("Available columns: date"));
    }

    // --- Color column tests ---

    #[test]
    fn test_color_column_auto_assigned() {
        let schema = make_schema(vec![
            ("date", DataType::Temporal),
            ("region", DataType::Categorical),
            ("revenue", DataType::Quantitative),
        ]);
        let rec = select_chart(&schema, Some("date"), Some("revenue")).unwrap();
        assert_eq!(rec.color_column.as_deref(), Some("region"));
    }

    #[test]
    fn test_no_color_when_categorical_used_as_axis() {
        let schema = make_schema(vec![
            ("city", DataType::Categorical),
            ("revenue", DataType::Quantitative),
        ]);
        let rec = select_chart(&schema, Some("city"), Some("revenue")).unwrap();
        assert_eq!(rec.color_column, None);
    }

    // --- Edge cases ---

    #[test]
    fn test_empty_schema_returns_error() {
        let schema = make_schema(vec![]);
        let rec = select_chart(&schema, None, None);
        assert!(rec.is_err());
        assert!(rec.unwrap_err().contains("No columns detected"));
    }

    #[test]
    fn test_only_nominal_returns_descriptive_error() {
        let schema = make_schema(vec![
            ("id", DataType::Nominal),
            ("description", DataType::Nominal),
        ]);
        let rec = select_chart(&schema, None, None);
        assert!(rec.is_err());
        let err = rec.unwrap_err();
        assert!(err.contains("id=Text"));
        assert!(err.contains("description=Text"));
        assert!(err.contains("Hint: specify axes with -x and -y"));
    }

    // --- Partial hint tests (y-only, x-only) ---

    #[test]
    fn test_y_only_hint_is_honored() {
        let schema = make_schema(vec![
            ("date", DataType::Temporal),
            ("city", DataType::Categorical),
            ("revenue", DataType::Quantitative),
            ("profit", DataType::Quantitative),
        ]);
        // User specifies only -y profit; should pick profit as Y, auto-select best X
        let rec = select_chart(&schema, None, Some("profit")).unwrap();
        assert_eq!(rec.y_column.as_deref(), Some("profit"));
        // date is temporal → best X candidate for line chart
        assert_eq!(rec.x_column, "date");
        assert_eq!(rec.chart_type, ChartType::Line);
    }

    #[test]
    fn test_y_only_hint_with_categorical_x() {
        let schema = make_schema(vec![
            ("city", DataType::Categorical),
            ("revenue", DataType::Quantitative),
            ("profit", DataType::Quantitative),
        ]);
        let rec = select_chart(&schema, None, Some("profit")).unwrap();
        assert_eq!(rec.y_column.as_deref(), Some("profit"));
        assert_eq!(rec.x_column, "city");
        assert_eq!(rec.chart_type, ChartType::Bar);
    }

    #[test]
    fn test_x_only_hint_is_honored() {
        let schema = make_schema(vec![
            ("date", DataType::Temporal),
            ("city", DataType::Categorical),
            ("revenue", DataType::Quantitative),
            ("profit", DataType::Quantitative),
        ]);
        // User specifies only -x city; should pick city as X, auto-select best Y
        let rec = select_chart(&schema, Some("city"), None).unwrap();
        assert_eq!(rec.x_column, "city");
        // first quantitative column should be Y
        assert_eq!(rec.y_column.as_deref(), Some("revenue"));
        assert_eq!(rec.chart_type, ChartType::Bar);
    }

    #[test]
    fn test_x_only_hint_temporal() {
        let schema = make_schema(vec![
            ("date", DataType::Temporal),
            ("city", DataType::Categorical),
            ("revenue", DataType::Quantitative),
        ]);
        let rec = select_chart(&schema, Some("date"), None).unwrap();
        assert_eq!(rec.x_column, "date");
        assert_eq!(rec.y_column.as_deref(), Some("revenue"));
        assert_eq!(rec.chart_type, ChartType::Line);
    }

    #[test]
    fn test_y_only_hint_nonexistent_column_errors() {
        let schema = make_schema(vec![
            ("date", DataType::Temporal),
            ("revenue", DataType::Quantitative),
        ]);
        let rec = select_chart(&schema, None, Some("nonexistent"));
        assert!(rec.is_err());
        assert!(rec.unwrap_err().contains("nonexistent"));
    }

    #[test]
    fn test_x_only_hint_nonexistent_column_errors() {
        let schema = make_schema(vec![
            ("date", DataType::Temporal),
            ("revenue", DataType::Quantitative),
        ]);
        let rec = select_chart(&schema, Some("nonexistent"), None);
        assert!(rec.is_err());
        assert!(rec.unwrap_err().contains("nonexistent"));
    }
}
