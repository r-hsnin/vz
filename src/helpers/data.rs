use anyhow::Result;

use crate::cli::{self, Cli};
use crate::infer::types::Schema;
use crate::loader::LoadedData;
use crate::{chart, filter};

use super::args::YOptions;

/// Parse and apply --where filters to loaded data.
pub fn apply_filters(data: LoadedData, filters: &[String]) -> Result<LoadedData> {
    if filters.is_empty() {
        return Ok(data);
    }
    let original_count = data.rows.len();
    let predicates: Vec<filter::Predicate> = filters
        .iter()
        .map(|expr| filter::parse_predicate(expr))
        .collect::<Result<Vec<_>>>()?;
    let filtered = filter::filter_data(data, &predicates)?;
    eprintln!(
        "info: filtered {}/{} rows ({})",
        filtered.rows.len(),
        original_count,
        filters.join(" & ")
    );
    Ok(filtered)
}

pub fn build_recommendation(
    cli: &Cli,
    schema: &Schema,
    y_opts: &YOptions,
) -> Result<chart::selector::ChartRecommendation> {
    let x_hint = cli
        .x_col
        .as_deref()
        .map(|s| crate::cli::parse_column_spec(s).0);
    let mut recommendation = chart::select_chart(schema, x_hint, y_opts.hint.as_deref())?;

    if cli.chart_type == Some(cli::ChartTypeArg::Bar) && cli.x_col.is_none() {
        adjust_bar_recommendation(&mut recommendation, schema);
    }

    if let Some(ref color) = cli.color_col {
        recommendation.color_column = Some(color.clone());
    }

    if !y_opts.extra_columns.is_empty() && cli.color_col.is_none() {
        recommendation.color_column = None;
    }

    Ok(recommendation)
}

/// When user overrides to bar chart, prefer a categorical column for X-axis.
pub fn adjust_bar_recommendation(
    recommendation: &mut chart::ChartRecommendation,
    schema: &crate::infer::types::Schema,
) {
    use crate::infer::types::DataType;

    let x_meta = schema
        .columns
        .iter()
        .find(|c| c.name == recommendation.x_column);
    if x_meta.map(|c| c.data_type) == Some(DataType::Categorical) {
        return;
    }

    let cat_cols = schema.columns_of_type(DataType::Categorical);
    if let Some(cat_col) = cat_cols.first() {
        recommendation.x_column = cat_col.name.clone();

        if recommendation.color_column.as_deref() == Some(cat_col.name.as_str()) {
            recommendation.color_column = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::selector::{ChartRecommendation, ChartType};
    use crate::cli::Cli;
    use crate::infer::types::{ColumnMeta, DataType, Schema};
    use crate::loader::LoadedData;
    use clap::Parser;

    use super::super::args::parse_y_options;

    fn make_schema(cols: &[(&str, DataType)]) -> Schema {
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

    fn make_recommendation(x: &str, y: Option<&str>, color: Option<&str>) -> ChartRecommendation {
        ChartRecommendation {
            chart_type: ChartType::Bar,
            x_column: x.to_string(),
            y_column: y.map(|s| s.to_string()),
            color_column: color.map(|s| s.to_string()),
        }
    }

    // --- apply_filters ---

    #[test]
    fn apply_filters_empty_filters_returns_unchanged() {
        let data = LoadedData {
            headers: vec!["city".into(), "revenue".into()],
            rows: vec![
                vec!["Tokyo".into(), "100".into()],
                vec!["Osaka".into(), "200".into()],
            ],
        };
        let filters: Vec<String> = vec![];
        let result = apply_filters(data, &filters).unwrap();
        assert_eq!(result.rows.len(), 2);
    }

    #[test]
    fn apply_filters_single_equality_filter() {
        let data = LoadedData {
            headers: vec!["city".into(), "revenue".into()],
            rows: vec![
                vec!["Tokyo".into(), "100".into()],
                vec!["Osaka".into(), "200".into()],
                vec!["Tokyo".into(), "300".into()],
            ],
        };
        let filters = vec!["city=Tokyo".to_string()];
        let result = apply_filters(data, &filters).unwrap();
        assert_eq!(result.rows.len(), 2);
        assert!(result.rows.iter().all(|r| r[0] == "Tokyo"));
    }

    #[test]
    fn apply_filters_invalid_filter_returns_error() {
        let data = LoadedData {
            headers: vec!["city".into()],
            rows: vec![],
        };
        let filters = vec!["no_operator_here".to_string()];
        let result = apply_filters(data, &filters);
        assert!(result.is_err());
    }

    // --- adjust_bar_recommendation ---

    #[test]
    fn adjust_bar_x_already_categorical_is_noop() {
        let schema = make_schema(&[
            ("city", DataType::Categorical),
            ("revenue", DataType::Quantitative),
        ]);
        let mut rec = make_recommendation("city", Some("revenue"), None);
        adjust_bar_recommendation(&mut rec, &schema);
        assert_eq!(rec.x_column, "city");
    }

    #[test]
    fn adjust_bar_quantitative_x_no_categorical_available() {
        let schema = make_schema(&[
            ("x_val", DataType::Quantitative),
            ("y_val", DataType::Quantitative),
        ]);
        let mut rec = make_recommendation("x_val", Some("y_val"), None);
        adjust_bar_recommendation(&mut rec, &schema);
        assert_eq!(rec.x_column, "x_val");
    }

    #[test]
    fn adjust_bar_swaps_temporal_to_categorical() {
        let schema = make_schema(&[
            ("date", DataType::Temporal),
            ("city", DataType::Categorical),
            ("revenue", DataType::Quantitative),
        ]);
        let mut rec = make_recommendation("date", Some("revenue"), None);
        adjust_bar_recommendation(&mut rec, &schema);
        assert_eq!(rec.x_column, "city");
    }

    #[test]
    fn adjust_bar_clears_color_when_matches_new_x() {
        let schema = make_schema(&[
            ("date", DataType::Temporal),
            ("city", DataType::Categorical),
            ("revenue", DataType::Quantitative),
        ]);
        let mut rec = make_recommendation("date", Some("revenue"), Some("city"));
        adjust_bar_recommendation(&mut rec, &schema);
        assert_eq!(rec.x_column, "city");
        assert_eq!(rec.color_column, None);
    }

    #[test]
    fn adjust_bar_preserves_color_when_different() {
        let schema = make_schema(&[
            ("date", DataType::Temporal),
            ("region", DataType::Categorical),
            ("city", DataType::Categorical),
            ("revenue", DataType::Quantitative),
        ]);
        let mut rec = make_recommendation("date", Some("revenue"), Some("city"));
        adjust_bar_recommendation(&mut rec, &schema);
        assert_eq!(rec.x_column, "region");
        assert_eq!(rec.color_column, Some("city".to_string()));
    }

    // --- build_recommendation ---

    #[test]
    fn build_recommendation_basic_temporal_quant() {
        let cli = Cli::try_parse_from(["vz", "data.csv", "-x", "month", "-y", "revenue"]).unwrap();
        let schema = make_schema(&[
            ("month", DataType::Temporal),
            ("revenue", DataType::Quantitative),
        ]);
        let y_opts = parse_y_options(&cli);
        let rec = build_recommendation(&cli, &schema, &y_opts).unwrap();
        assert_eq!(rec.x_column, "month");
        assert_eq!(rec.y_column, Some("revenue".to_string()));
    }

    #[test]
    fn build_recommendation_color_col_overrides() {
        let cli = Cli::try_parse_from([
            "vz", "data.csv", "-x", "month", "-y", "revenue", "-c", "region",
        ])
        .unwrap();
        let schema = make_schema(&[
            ("month", DataType::Temporal),
            ("revenue", DataType::Quantitative),
            ("region", DataType::Categorical),
        ]);
        let y_opts = parse_y_options(&cli);
        let rec = build_recommendation(&cli, &schema, &y_opts).unwrap();
        assert_eq!(rec.color_column, Some("region".to_string()));
    }

    #[test]
    fn build_recommendation_extra_y_clears_color() {
        let cli =
            Cli::try_parse_from(["vz", "data.csv", "-x", "month", "-y", "revenue,profit"]).unwrap();
        let schema = make_schema(&[
            ("month", DataType::Temporal),
            ("revenue", DataType::Quantitative),
            ("profit", DataType::Quantitative),
        ]);
        let y_opts = parse_y_options(&cli);
        let rec = build_recommendation(&cli, &schema, &y_opts).unwrap();
        assert_eq!(rec.color_column, None);
    }
}
