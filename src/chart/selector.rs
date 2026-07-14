use anyhow::{Result, bail};

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
) -> Result<ChartRecommendation> {
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
fn validate_column<'a>(schema: &'a Schema, name: &str) -> Result<&'a ColumnMeta> {
    schema.find_column(name).ok_or_else(|| {
        anyhow::anyhow!(
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
fn select_with_y_hint(schema: &Schema, y_name: &str) -> Result<ChartRecommendation> {
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
fn select_with_x_hint(schema: &Schema, x_name: &str) -> Result<ChartRecommendation> {
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

    bail!(
        "Cannot find a suitable Y axis column to pair with '{}'. \
         Hint: add a numeric column or specify -y explicitly.",
        x_name
    )
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
fn auto_select(schema: &Schema) -> Result<ChartRecommendation> {
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

    bail!("{}", no_chart_error(schema))
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
#[path = "selector_tests.rs"]
mod tests;
