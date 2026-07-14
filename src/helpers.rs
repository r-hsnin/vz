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
fn effective_agg(
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
