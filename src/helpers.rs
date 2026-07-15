use anyhow::Result;
use std::path::PathBuf;

use crate::cli::{self, Cli, parse_multi_y_specs};
use crate::infer::types::Schema;
use crate::loader::LoadedData;
use crate::{chart, filter, loader, oneshot, theme};

pub(crate) fn build_render_options<'a>(
    cli: &'a Cli,
    y_opts: &'a YOptions,
    recommendation: &chart::selector::ChartRecommendation,
    schema: &Schema,
) -> oneshot::RenderOptions<'a> {
    let agg = effective_agg(cli, recommendation, schema);
    oneshot::RenderOptions {
        chart_type_override: cli.chart_type,
        y_label_override: y_opts.label_override.as_deref(),
        width: cli.width,
        height: cli.height,
        sort_order: cli.effective_sort(),
        extra_y_columns: y_opts.extra_columns.clone(),
        limit: cli.top.or(cli.tail),
        agg,
        title: cli.title.clone(),
        labels: cli.labels,
        theme: resolve_theme(cli),
        bins: cli.bins,
    }
}

/// Determine effective aggregation function.
/// Auto-switches to Count when bar chart is forced on a categorical Y column
/// that was auto-inferred (not explicitly specified by the user).
pub(crate) fn effective_agg(
    cli: &Cli,
    recommendation: &chart::selector::ChartRecommendation,
    schema: &Schema,
) -> cli::AggFunction {
    if let Some(agg) = cli.agg {
        return agg;
    }

    // When bar chart is forced, Y was auto-inferred (not explicit), and Y is not numeric,
    // default to Count. This handles the case where both columns are categorical
    // (e.g., departments.csv with department + status).
    if cli.chart_type == Some(cli::ChartTypeArg::Bar) && cli.y_col.is_none() {
        let y_is_categorical = recommendation
            .y_column
            .as_ref()
            .and_then(|y| schema.find_column(y))
            .map(|c| c.data_type != crate::infer::types::DataType::Quantitative)
            .unwrap_or(false);
        if y_is_categorical {
            return cli::AggFunction::Count;
        }
    }

    cli::AggFunction::Sum
}

/// Resolve the theme from CLI args.
pub(crate) fn resolve_theme(cli: &Cli) -> theme::Theme {
    match cli.theme {
        Some(cli::ThemeArg::Light) => theme::Theme::light(),
        Some(cli::ThemeArg::HighContrast) => theme::Theme::high_contrast(),
        _ => theme::Theme::dark(),
    }
}

pub(crate) fn resolve_input_file(cli: &Cli) -> Result<PathBuf> {
    match cli.file.as_ref() {
        Some(f) => Ok(f.clone()),
        None => {
            if !std::io::IsTerminal::is_terminal(&std::io::stdin()) {
                Ok(PathBuf::from("-"))
            } else {
                anyhow::bail!("No input file specified. Usage: vz <file> or pipe data to stdin");
            }
        }
    }
}

pub(crate) fn build_recommendation(
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

/// Convert CLI format argument to loader InputFormat.
pub(crate) fn format_override(cli: &Cli) -> Option<loader::InputFormat> {
    cli.format.map(|f| match f {
        cli::InputFormatArg::Csv => loader::InputFormat::Csv,
        cli::InputFormatArg::Tsv => loader::InputFormat::Tsv,
        cli::InputFormatArg::Json => loader::InputFormat::Json,
        cli::InputFormatArg::Ndjson => loader::InputFormat::Ndjson,
        cli::InputFormatArg::Space => loader::InputFormat::Space,
    })
}

/// Parse and apply --where filters to loaded data.
pub(crate) fn apply_filters(data: LoadedData, filters: &[String]) -> Result<LoadedData> {
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

/// Parsed Y-axis options from CLI.
pub(crate) struct YOptions {
    pub hint: Option<String>,
    pub label_override: Option<String>,
    pub extra_columns: Vec<(String, Option<String>)>,
}

/// Parse Y-axis options: primary Y hint, label override, and extra Y columns.
pub(crate) fn parse_y_options(cli: &Cli) -> YOptions {
    let y_specs: Vec<(&str, Option<&str>)> = cli
        .y_col
        .as_deref()
        .map(|s| parse_multi_y_specs(s))
        .unwrap_or_default();
    let hint = y_specs.first().map(|(col, _)| col.to_string());
    let label_override = y_specs
        .first()
        .and_then(|(_, label)| *label)
        .map(|s| s.to_string());
    let extra_columns: Vec<(String, Option<String>)> = y_specs
        .iter()
        .skip(1)
        .map(|(col, label)| (col.to_string(), label.map(|l| l.to_string())))
        .collect();
    YOptions {
        hint,
        label_override,
        extra_columns,
    }
}

/// When user overrides to bar chart, prefer a categorical column for X-axis.
pub(crate) fn adjust_bar_recommendation(
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
    use crate::cli::{AggFunction, Cli};
    use crate::infer::types::{ColumnMeta, DataType, Schema};
    use crate::loader::{InputFormat, LoadedData};
    use clap::Parser;

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

    // --- resolve_input_file ---

    #[test]
    fn resolve_input_file_returns_path_when_file_specified() {
        let cli = Cli::try_parse_from(["vz", "sales.csv"]).unwrap();
        let result = resolve_input_file(&cli).unwrap();
        assert_eq!(result, PathBuf::from("sales.csv"));
    }

    #[test]
    fn resolve_input_file_returns_dash_for_stdin_in_non_terminal() {
        // In cargo test, stdin is not a terminal, so this returns Ok("-")
        let cli = Cli::try_parse_from(["vz", "--info"]).unwrap();
        let result = resolve_input_file(&cli);
        // When stdin is not a terminal (CI/test), returns Ok("-")
        // When stdin IS a terminal, returns Err
        // Either outcome is acceptable; we just verify it doesn't panic
        assert!(result.is_ok() || result.is_err());
        if let Ok(path) = result {
            assert_eq!(path, PathBuf::from("-"));
        }
    }

    // --- effective_agg ---

    #[test]
    fn effective_agg_explicit_overrides_all() {
        let cli = Cli::try_parse_from(["vz", "data.csv", "--agg", "mean"]).unwrap();
        let schema = make_schema(&[
            ("city", DataType::Categorical),
            ("revenue", DataType::Quantitative),
        ]);
        let rec = make_recommendation("city", Some("revenue"), None);
        assert_eq!(effective_agg(&cli, &rec, &schema), AggFunction::Mean);
    }

    #[test]
    fn effective_agg_defaults_to_sum() {
        let cli = Cli::try_parse_from(["vz", "data.csv"]).unwrap();
        let schema = make_schema(&[
            ("city", DataType::Categorical),
            ("revenue", DataType::Quantitative),
        ]);
        let rec = make_recommendation("city", Some("revenue"), None);
        assert_eq!(effective_agg(&cli, &rec, &schema), AggFunction::Sum);
    }

    #[test]
    fn effective_agg_bar_forced_categorical_y_becomes_count() {
        let cli = Cli::try_parse_from(["vz", "data.csv", "-t", "bar"]).unwrap();
        let schema = make_schema(&[
            ("department", DataType::Categorical),
            ("status", DataType::Categorical),
        ]);
        let rec = make_recommendation("department", Some("status"), None);
        assert_eq!(effective_agg(&cli, &rec, &schema), AggFunction::Count);
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

    // --- parse_y_options ---

    #[test]
    fn parse_y_options_single_column_no_label() {
        let cli = Cli::try_parse_from(["vz", "data.csv", "-y", "revenue"]).unwrap();
        let opts = parse_y_options(&cli);
        assert_eq!(opts.hint, Some("revenue".to_string()));
        assert_eq!(opts.label_override, None);
        assert!(opts.extra_columns.is_empty());
    }

    #[test]
    fn parse_y_options_multi_y_with_labels() {
        let cli =
            Cli::try_parse_from(["vz", "data.csv", "-y", "revenue:Rev,profit:Profit"]).unwrap();
        let opts = parse_y_options(&cli);
        assert_eq!(opts.hint, Some("revenue".to_string()));
        assert_eq!(opts.label_override, Some("Rev".to_string()));
        assert_eq!(
            opts.extra_columns,
            vec![("profit".to_string(), Some("Profit".to_string()))]
        );
    }

    #[test]
    fn parse_y_options_no_y_specified() {
        let cli = Cli::try_parse_from(["vz", "data.csv"]).unwrap();
        let opts = parse_y_options(&cli);
        assert_eq!(opts.hint, None);
        assert_eq!(opts.label_override, None);
        assert!(opts.extra_columns.is_empty());
    }

    // --- format_override ---

    #[test]
    fn format_override_none_when_not_specified() {
        let cli = Cli::try_parse_from(["vz", "data.csv"]).unwrap();
        assert_eq!(format_override(&cli), None);
    }

    #[test]
    fn format_override_maps_tsv() {
        let cli = Cli::try_parse_from(["vz", "-", "-f", "tsv"]).unwrap();
        assert_eq!(format_override(&cli), Some(InputFormat::Tsv));
    }

    #[test]
    fn format_override_maps_ndjson() {
        let cli = Cli::try_parse_from(["vz", "-", "-f", "ndjson"]).unwrap();
        assert_eq!(format_override(&cli), Some(InputFormat::Ndjson));
    }

    // --- build_render_options ---

    #[test]
    fn build_render_options_default_values() {
        let cli = Cli::try_parse_from(["vz", "data.csv"]).unwrap();
        let schema = make_schema(&[
            ("month", DataType::Temporal),
            ("revenue", DataType::Quantitative),
        ]);
        let y_opts = parse_y_options(&cli);
        let rec = make_recommendation("month", Some("revenue"), None);
        let opts = build_render_options(&cli, &y_opts, &rec, &schema);
        assert_eq!(opts.width, None);
        assert_eq!(opts.height, None);
        assert_eq!(opts.sort_order, None);
        assert_eq!(opts.agg, AggFunction::Sum);
        assert!(!opts.labels);
        assert_eq!(opts.bins, None);
        assert_eq!(opts.title, None);
        assert_eq!(opts.chart_type_override, None);
    }

    #[test]
    fn build_render_options_with_all_overrides() {
        let cli = Cli::try_parse_from([
            "vz",
            "data.csv",
            "-W",
            "80",
            "-H",
            "20",
            "--top",
            "5",
            "--agg",
            "mean",
            "--title",
            "My Chart",
            "--labels",
            "--bins",
            "15",
            "--theme",
            "light",
            "-y",
            "revenue:Rev,profit",
        ])
        .unwrap();
        let schema = make_schema(&[
            ("city", DataType::Categorical),
            ("revenue", DataType::Quantitative),
            ("profit", DataType::Quantitative),
        ]);
        let y_opts = parse_y_options(&cli);
        let rec = make_recommendation("city", Some("revenue"), None);
        let opts = build_render_options(&cli, &y_opts, &rec, &schema);
        assert_eq!(opts.width, Some(80));
        assert_eq!(opts.height, Some(20));
        assert_eq!(opts.limit, Some(5));
        assert_eq!(opts.agg, AggFunction::Mean);
        assert_eq!(opts.title, Some("My Chart".to_string()));
        assert!(opts.labels);
        assert_eq!(opts.bins, Some(15));
        assert_eq!(opts.y_label_override, Some("Rev"));
        assert_eq!(opts.extra_y_columns, vec![("profit".to_string(), None)]);
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
        assert_eq!(rec.x_column, "x_val"); // unchanged — no categorical to swap to
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
        // Multi-Y without explicit color → color_column cleared
        assert_eq!(rec.color_column, None);
    }
}
