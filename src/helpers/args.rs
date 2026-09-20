use crate::cli::{self, Cli, parse_multi_y_specs};
use crate::infer::types::Schema;
use crate::{chart, oneshot};

pub fn build_render_options<'a>(
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
        theme: crate::cli::resolve_theme(cli),
        bins: cli.bins,
    }
}

/// Determine effective aggregation function.
/// Auto-switches to Count when bar chart is forced on a categorical Y column
/// that was auto-inferred (not explicitly specified by the user).
pub fn effective_agg(
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
    // Self-aggregation (`-x city` alone → x=y=city) also counts rows per category.
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

    // Categorical X with no quantitative Y (incl. `-x city` alone):
    // count rows per category (matches the README "count auto-applied" claim).
    if cli.y_col.is_none()
        && let Some(x_name) = cli
            .x_col
            .as_deref()
            .map(|s| crate::cli::parse_column_spec(s).0)
        && let Some(x_meta) = schema.find_column(x_name)
        && x_meta.data_type == crate::infer::types::DataType::Categorical
        && schema
            .columns_of_type(crate::infer::types::DataType::Quantitative)
            .is_empty()
    {
        return cli::AggFunction::Count;
    }

    cli::AggFunction::Sum
}

/// Parsed Y-axis options from CLI.
pub struct YOptions {
    pub hint: Option<String>,
    pub label_override: Option<String>,
    pub extra_columns: Vec<(String, Option<String>)>,
}

/// Parse Y-axis options: primary Y hint, label override, and extra Y columns.
pub fn parse_y_options(cli: &Cli) -> YOptions {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::selector::ChartType;
    use crate::cli::{AggFunction, Cli};
    use crate::infer::types::DataType;
    use crate::test_helpers::{make_recommendation, make_schema};
    use clap::Parser;

    // --- effective_agg ---

    #[test]
    fn effective_agg_explicit_overrides_all() {
        let cli = Cli::try_parse_from(["vz", "data.csv", "--agg", "mean"]).unwrap();
        let schema = make_schema(&[
            ("city", DataType::Categorical),
            ("revenue", DataType::Quantitative),
        ]);
        let rec = make_recommendation(ChartType::Bar, "city", Some("revenue"), None);
        assert_eq!(effective_agg(&cli, &rec, &schema), AggFunction::Mean);
    }

    #[test]
    fn effective_agg_defaults_to_sum() {
        let cli = Cli::try_parse_from(["vz", "data.csv"]).unwrap();
        let schema = make_schema(&[
            ("city", DataType::Categorical),
            ("revenue", DataType::Quantitative),
        ]);
        let rec = make_recommendation(ChartType::Bar, "city", Some("revenue"), None);
        assert_eq!(effective_agg(&cli, &rec, &schema), AggFunction::Sum);
    }

    #[test]
    fn effective_agg_bar_forced_categorical_y_becomes_count() {
        let cli = Cli::try_parse_from(["vz", "data.csv", "-t", "bar"]).unwrap();
        let schema = make_schema(&[
            ("department", DataType::Categorical),
            ("status", DataType::Categorical),
        ]);
        let rec = make_recommendation(ChartType::Bar, "department", Some("status"), None);
        assert_eq!(effective_agg(&cli, &rec, &schema), AggFunction::Count);
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

    // --- build_render_options ---

    #[test]
    fn build_render_options_default_values() {
        let cli = Cli::try_parse_from(["vz", "data.csv"]).unwrap();
        let schema = make_schema(&[
            ("month", DataType::Temporal),
            ("revenue", DataType::Quantitative),
        ]);
        let y_opts = parse_y_options(&cli);
        let rec = make_recommendation(ChartType::Bar, "month", Some("revenue"), None);
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
        let rec = make_recommendation(ChartType::Bar, "city", Some("revenue"), None);
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
}
