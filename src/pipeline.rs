//! Post-load data pipeline: filter → sample → validate → infer → render.

use anyhow::Result;
use std::path::Path;

use crate::chart::ChartRecommendation;
use crate::cli::{self, Cli};
use crate::helpers::{
    YOptions, apply_filters, build_recommendation, build_render_options, effective_agg,
    parse_y_options,
};
use crate::infer;
use crate::infer::types::Schema;
use crate::loader::{self, LoadedData};
use crate::oneshot;
use crate::output;

/// Infer schema from loaded data (eliminates boilerplate in multiple call sites).
///
/// Only passes the first [`SAMPLE_SIZE`](crate::infer::detector::SAMPLE_SIZE)
/// rows to inference. This avoids allocating a full `Vec<Vec<&str>>` for large datasets.
pub fn infer_from_data(data: &LoadedData) -> Schema {
    use crate::infer::detector::SAMPLE_SIZE;
    let headers: Vec<&str> = data.headers.iter().map(|s| s.as_str()).collect();
    let row_limit = data.rows.len().min(SAMPLE_SIZE);
    let rows: Vec<Vec<&str>> = data.rows[..row_limit]
        .iter()
        .map(|r| r.iter().map(|s| s.as_str()).collect())
        .collect();
    infer::infer_schema(&headers, &rows)
}

/// Shared post-load pipeline: filter → sample → validate → infer → render.
/// Used by both single-file and directory modes.
pub fn render_data(cli: &Cli, data: LoadedData, file: &Path) -> Result<()> {
    let pre_filter_count = data.rows.len();
    let data = apply_filters(data, &cli.filter)?;
    let data = if let Some(max_rows) = cli.sample {
        if max_rows == 0 {
            anyhow::bail!("--sample must be at least 1");
        }
        loader::apply_sampling(data, max_rows)
    } else {
        data
    };

    validate_loaded_data(&data, file, &cli.filter, pre_filter_count)?;

    // Validate -c column exists in the loaded data
    if let Some(ref color_col) = cli.color_col
        && !data.headers.iter().any(|h| h == color_col)
    {
        anyhow::bail!(
            "Color column '{}' not found. Available columns: {}",
            color_col,
            data.headers.join(", ")
        );
    }

    let schema = infer_from_data(&data);

    if cli.info {
        if cli.output == Some(cli::OutputFormat::Json) {
            crate::info::print_info_json(file, &data, &schema)?;
        } else {
            crate::info::print_info(file, &data, &schema);
        }
        return Ok(());
    }

    let mut y_opts = parse_y_options(cli);
    let recommendation = build_recommendation(cli, &schema, &y_opts)?;
    if cli.all_y {
        expand_all_y(&recommendation, &schema, &mut y_opts);
    }

    if cli.output == Some(cli::OutputFormat::Json) {
        print_chart_json(file, &data, &schema, &recommendation, cli, &y_opts)?;
        return Ok(());
    }

    dispatch_output(
        cli,
        &recommendation,
        &data.headers,
        &data.rows,
        &y_opts,
        &schema,
    )
}

/// Validate that loaded data is non-empty and produce clear error messages.
fn validate_loaded_data(
    data: &LoadedData,
    file: &Path,
    filters: &[String],
    pre_filter_count: usize,
) -> Result<()> {
    if data.rows.is_empty() {
        if !filters.is_empty() && pre_filter_count > 0 {
            anyhow::bail!(
                "No rows remain after filtering. All {} rows were excluded by --where predicates.",
                pre_filter_count,
            );
        }
        if data.headers.is_empty() || data.headers.iter().all(|h| h.is_empty()) {
            anyhow::bail!(
                "Input '{}' is empty — no data to visualize.\n\n  Tip: ensure the command or file produces output before piping to vz.",
                file.display(),
            );
        }
        anyhow::bail!(
            "No data rows found in '{}'. The file appears to contain only headers.",
            file.display(),
        );
    }
    Ok(())
}

/// Dispatch to the appropriate output renderer based on CLI flags.
fn dispatch_output(
    cli: &Cli,
    recommendation: &ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
    y_opts: &YOptions,
    schema: &Schema,
) -> Result<()> {
    match cli.output {
        Some(cli::OutputFormat::Table) => {
            output::table::print_table(recommendation, headers, rows, cli)?;
        }
        Some(cli::OutputFormat::Spark) => {
            print_spark(recommendation, headers, rows, cli, schema);
        }
        Some(cli::OutputFormat::Svg) => {
            let opts = build_render_options(cli, y_opts, recommendation, schema);
            output::svg::print_svg(recommendation, headers, rows, &opts)?;
        }
        Some(cli::OutputFormat::Html) => {
            let opts = build_render_options(cli, y_opts, recommendation, schema);
            output::html::print_html(recommendation, headers, rows, &opts)?;
        }
        Some(cli::OutputFormat::Markdown) => {
            output::markdown::print_markdown(recommendation, headers, rows, cli, schema)?;
        }
        _ => {
            let opts = build_render_options(cli, y_opts, recommendation, schema);
            oneshot::render_oneshot(recommendation, headers, rows, &opts)?;
        }
    }
    Ok(())
}

/// Print sparkline output (delegates to output::spark module).
fn print_spark(
    recommendation: &ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
    cli: &Cli,
    schema: &Schema,
) {
    let params = output::spark::SparkParams {
        chart_type_override: cli.chart_type,
        agg: effective_agg(cli, recommendation, schema),
        sort: cli.effective_sort(),
        limit: cli.top.or(cli.tail),
        color_col: cli.color_col.clone(),
        bins: cli.bins,
    };
    output::spark::print_spark(recommendation, headers, rows, &params);
}

/// Expand `--all-y`: add all remaining quantitative columns to extra_y.
fn expand_all_y(recommendation: &ChartRecommendation, schema: &Schema, y_opts: &mut YOptions) {
    let x_col = &recommendation.x_column;
    let primary_y = recommendation.y_column.as_deref().unwrap_or("");
    let extra: Vec<(String, Option<String>)> = schema
        .columns
        .iter()
        .filter(|c| c.data_type == infer::types::DataType::Quantitative)
        .filter(|c| c.name != *x_col && c.name != primary_y)
        .filter(|c| !y_opts.extra_columns.iter().any(|(n, _)| n == &c.name))
        .map(|c| (c.name.clone(), None))
        .collect();
    y_opts.extra_columns.extend(extra);
}

/// Print chart data as JSON — delegates to output::chart_json module.
fn print_chart_json(
    file: &Path,
    data: &LoadedData,
    schema: &Schema,
    recommendation: &ChartRecommendation,
    cli: &Cli,
    y_opts: &YOptions,
) -> anyhow::Result<()> {
    let params = output::chart_json::ChartJsonParams {
        chart_type: cli
            .chart_type
            .map(|ct| ct.to_chart_type())
            .unwrap_or(recommendation.chart_type),
        sort: cli.effective_sort(),
        agg: effective_agg(cli, recommendation, schema),
        limit: cli.top.or(cli.tail),
        extra_y_columns: y_opts.extra_columns.clone(),
        color_column: cli.color_col.clone(),
        bins: cli.bins,
    };
    output::chart_json::print_chart_json(
        file,
        data,
        schema,
        recommendation,
        &data.headers,
        &data.rows,
        &params,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn loaded(headers: &[&str], rows: &[&[&str]]) -> LoadedData {
        LoadedData {
            headers: headers.iter().map(|s| s.to_string()).collect(),
            rows: rows
                .iter()
                .map(|r| r.iter().map(|s| s.to_string()).collect())
                .collect(),
        }
    }

    #[test]
    fn validate_loaded_data_empty() {
        let data = loaded(&[], &[]);
        let err = validate_loaded_data(&data, &PathBuf::from("in.csv"), &[], 0).unwrap_err();
        assert!(err.to_string().contains("is empty"), "{err}");
    }

    #[test]
    fn validate_loaded_data_all_filtered_out() {
        let data = loaded(&["a"], &[]);
        let err = validate_loaded_data(&data, &PathBuf::from("in.csv"), &["a>1".to_string()], 5)
            .unwrap_err();
        assert!(
            err.to_string().contains("No rows remain after filtering"),
            "{err}"
        );
    }

    #[test]
    fn validate_loaded_data_headers_only() {
        let data = loaded(&["a", "b"], &[]);
        let err = validate_loaded_data(&data, &PathBuf::from("in.csv"), &[], 0).unwrap_err();
        assert!(err.to_string().contains("only headers"), "{err}");
    }

    #[test]
    fn validate_loaded_data_blank_headers_treated_as_empty() {
        let data = loaded(&["", ""], &[]);
        let err = validate_loaded_data(&data, &PathBuf::from("in.csv"), &[], 0).unwrap_err();
        assert!(err.to_string().contains("is empty"), "{err}");
    }

    #[test]
    fn validate_loaded_data_ok_with_rows() {
        let data = loaded(&["a"], &[&["1"]]);
        assert!(validate_loaded_data(&data, &PathBuf::from("in.csv"), &[], 1).is_ok());
    }
}
