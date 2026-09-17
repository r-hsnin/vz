use super::*;
use crate::test_helpers::make_schema;

// --- Auto selection tests ---

#[test]
fn test_temporal_quantitative_gives_line() {
    let schema = make_schema(&[
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
    let schema = make_schema(&[
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
    let schema = make_schema(&[
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
    let schema = make_schema(&[("age", DataType::Quantitative)]);
    let rec = select_chart(&schema, None, None).unwrap();
    assert_eq!(rec.chart_type, ChartType::Histogram);
    assert_eq!(rec.x_column, "age");
    assert_eq!(rec.y_column, None);
}

#[test]
fn test_two_categorical_gives_bar() {
    let schema = make_schema(&[
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
    let schema = make_schema(&[
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
    let schema = make_schema(&[
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
    let schema = make_schema(&[("date", DataType::Temporal)]);
    let rec = select_chart(&schema, Some("nonexistent"), Some("also_bad"));
    assert!(rec.is_err());
    let err = rec.unwrap_err().to_string();
    assert!(err.contains("nonexistent"));
    assert!(err.contains("Available columns: date"));
}

#[test]
fn test_nonexistent_column_suggests_close_match() {
    let schema = make_schema(&[
        ("date", DataType::Temporal),
        ("revenue", DataType::Quantitative),
    ]);
    let err = select_chart(&schema, Some("date"), Some("revnue"))
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("Did you mean 'revenue'?"),
        "expected suggestion, got: {err}"
    );
}

#[test]
fn test_nonexistent_column_marks_case_sensitivity() {
    let schema = make_schema(&[
        ("date", DataType::Temporal),
        ("revenue", DataType::Quantitative),
    ]);
    let err = select_chart(&schema, Some("date"), Some("Revenue"))
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("Did you mean 'revenue'? Note: column names are case-sensitive."),
        "expected case note, got: {err}"
    );
}

// --- Color column tests ---

#[test]
fn test_color_column_auto_assigned() {
    let schema = make_schema(&[
        ("date", DataType::Temporal),
        ("region", DataType::Categorical),
        ("revenue", DataType::Quantitative),
    ]);
    let rec = select_chart(&schema, Some("date"), Some("revenue")).unwrap();
    assert_eq!(rec.color_column.as_deref(), Some("region"));
}

#[test]
fn test_no_color_when_categorical_used_as_axis() {
    let schema = make_schema(&[
        ("city", DataType::Categorical),
        ("revenue", DataType::Quantitative),
    ]);
    let rec = select_chart(&schema, Some("city"), Some("revenue")).unwrap();
    assert_eq!(rec.color_column, None);
}

// --- Edge cases ---

#[test]
fn test_empty_schema_returns_error() {
    let schema = make_schema(&[]);
    let rec = select_chart(&schema, None, None);
    assert!(rec.is_err());
    assert!(rec.unwrap_err().to_string().contains("No columns detected"));
}

#[test]
fn test_only_nominal_returns_descriptive_error() {
    let schema = make_schema(&[
        ("id", DataType::Nominal),
        ("description", DataType::Nominal),
    ]);
    let rec = select_chart(&schema, None, None);
    assert!(rec.is_err());
    let err = rec.unwrap_err().to_string();
    assert!(err.contains("id=Text"));
    assert!(err.contains("description=Text"));
    assert!(err.contains("Hint: specify axes with -x and -y"));
}

// --- Partial hint tests (y-only, x-only) ---

#[test]
fn test_y_only_hint_is_honored() {
    let schema = make_schema(&[
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
    let schema = make_schema(&[
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
    let schema = make_schema(&[
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
    let schema = make_schema(&[
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
    let schema = make_schema(&[
        ("date", DataType::Temporal),
        ("revenue", DataType::Quantitative),
    ]);
    let rec = select_chart(&schema, None, Some("nonexistent"));
    assert!(rec.is_err());
    assert!(rec.unwrap_err().to_string().contains("nonexistent"));
}

#[test]
fn test_x_only_hint_nonexistent_column_errors() {
    let schema = make_schema(&[
        ("date", DataType::Temporal),
        ("revenue", DataType::Quantitative),
    ]);
    let rec = select_chart(&schema, Some("nonexistent"), None);
    assert!(rec.is_err());
    assert!(rec.unwrap_err().to_string().contains("nonexistent"));
}

// --- Fallback warning tests ---

#[test]
fn test_fallback_warning_for_nominal_pair() {
    let schema = make_schema(&[
        ("item", DataType::Categorical),
        ("price", DataType::Nominal),
    ]);
    let rec = select_chart(&schema, Some("item"), Some("price")).unwrap();
    assert_eq!(rec.chart_type, ChartType::Bar);
    let warning = fallback_warning(
        &schema,
        &rec.x_column,
        rec.y_column.as_deref(),
        rec.chart_type,
    )
    .expect("nominal pair must warn");
    assert!(warning.contains("falling back to bar"), "got: {warning}");
    assert!(warning.contains("price"), "got: {warning}");
}

#[test]
fn test_fallback_warning_none_for_first_class_pair() {
    let schema = make_schema(&[
        ("date", DataType::Temporal),
        ("revenue", DataType::Quantitative),
    ]);
    let rec = select_chart(&schema, Some("date"), Some("revenue")).unwrap();
    assert_eq!(rec.chart_type, ChartType::Line);
    assert_eq!(
        fallback_warning(
            &schema,
            &rec.x_column,
            rec.y_column.as_deref(),
            rec.chart_type
        ),
        None
    );
}

#[test]
fn test_fallback_warning_none_for_plain_bar() {
    let schema = make_schema(&[
        ("city", DataType::Categorical),
        ("revenue", DataType::Quantitative),
    ]);
    let rec = select_chart(&schema, Some("city"), Some("revenue")).unwrap();
    assert_eq!(rec.chart_type, ChartType::Bar);
    assert_eq!(
        fallback_warning(
            &schema,
            &rec.x_column,
            rec.y_column.as_deref(),
            rec.chart_type
        ),
        None
    );
}

#[test]
fn test_fallback_warning_for_temporal_pair_fallback() {
    // (Temporal, Temporal) has no chart rule and falls back to Bar — must warn.
    let schema = make_schema(&[("start", DataType::Temporal), ("end", DataType::Temporal)]);
    let rec = select_chart(&schema, Some("start"), Some("end")).unwrap();
    assert_eq!(rec.chart_type, ChartType::Bar);
    let warning = fallback_warning(
        &schema,
        &rec.x_column,
        rec.y_column.as_deref(),
        rec.chart_type,
    )
    .expect("catch-all bar fallback must warn");
    assert!(warning.contains("falling back to bar"), "got: {warning}");
}
