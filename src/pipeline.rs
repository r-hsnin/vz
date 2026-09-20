//! Post-load data pipeline: filter → sample → validate → infer → render.

use anyhow::Result;
use std::path::Path;

use crate::chart::{ChartRecommendation, recommend};
use crate::cli::{self, Cli, PipelineParams};
use crate::filter::apply_filters;
use crate::infer;
use crate::infer::types::Schema;
use crate::loader::{self, LoadedData};
use crate::oneshot;
use crate::output;

/// Infer schema from loaded data (eliminates boilerplate in multiple call sites).
///
/// Samples rows evenly across the whole dataset (head + tail) instead of only
/// the first [`SAMPLE_SIZE`](crate::infer::detector::SAMPLE_SIZE) rows, so a
/// file whose leading rows are all one type (e.g. 100 dates followed by 100
/// garbage strings) infers the same as the reversed file. Avoids allocating
/// a full `Vec<Vec<&str>>` for large datasets by capping sampled rows.
pub fn infer_from_data(data: &LoadedData) -> Schema {
    use crate::infer::detector::SAMPLE_SIZE;
    let headers: Vec<&str> = data.headers.iter().map(|s| s.as_str()).collect();
    let rows: Vec<Vec<&str>> = if data.rows.len() <= SAMPLE_SIZE {
        data.rows
            .iter()
            .map(|r| r.iter().map(|s| s.as_str()).collect())
            .collect()
    } else {
        // Evenly spaced indices covering head→tail (first and last always kept).
        let step = (data.rows.len() - 1) as f64 / (SAMPLE_SIZE - 1) as f64;
        (0..SAMPLE_SIZE)
            .map(|i| {
                let idx = (step * i as f64).round() as usize;
                data.rows[idx.min(data.rows.len() - 1)]
                    .iter()
                    .map(|s| s.as_str())
                    .collect()
            })
            .collect()
    };
    infer::infer_schema(&headers, &rows)
}

/// Shared post-load pipeline: filter → sample → validate → infer → render.
/// Used by both single-file and directory modes.
///
/// Takes the resolved [`PipelineParams`] (app plane builds it once via
/// `Cli::to_pipeline_params`); pure per-output resolution happens inside.
pub fn render_data(params: &PipelineParams, data: LoadedData, file: &Path) -> Result<()> {
    let pre_filter_count = data.rows.len();
    let outcome = apply_filters(data, &params.filters)?;
    if let Some(notice) = outcome.notice {
        eprintln!("{notice}");
    }
    let data = outcome.data;
    let data = if let Some(max_rows) = params.sample {
        if max_rows == 0 {
            anyhow::bail!("--sample must be at least 1");
        }
        loader::apply_sampling(data, max_rows)
    } else {
        data
    };

    validate_loaded_data(&data, file, &params.filters, pre_filter_count)?;

    // Validate -c column exists in the loaded data
    validate_color_column(&data.headers, params.query.color_col.as_deref())?;

    let schema = infer_from_data(&data);

    if params.info {
        if params.output == Some(cli::OutputFormat::Json) {
            crate::info::print_info_json(file, &data, &schema)?;
        } else {
            crate::info::print_info(file, &data, &schema);
        }
        return Ok(());
    }

    let query = &params.query;
    let mut y_opts = recommend::parse_y_options(query.y_col.as_deref());
    let (recommendation, warnings) = recommend::build_recommendation(query, &schema, &y_opts)?;
    for w in warnings.0 {
        eprintln!("{w}");
    }
    if params.all_y {
        expand_all_y(&recommendation, &schema, &mut y_opts);
    }

    if params.output == Some(cli::OutputFormat::Json) {
        print_chart_json(file, &data, &schema, &recommendation, params, &y_opts)?;
        return Ok(());
    }

    dispatch_output(
        params,
        &recommendation,
        &data.headers,
        &data.rows,
        &y_opts,
        &schema,
    )
}

/// Cli adapter for [`render_data`]: converts once, then delegates.
pub fn render_data_from_cli(cli: &Cli, data: LoadedData, file: &Path) -> Result<()> {
    render_data(&cli.to_pipeline_params(), data, file)
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

/// Validate the `-c` color column against loaded headers, with typo hints.
pub(crate) fn validate_color_column(headers: &[String], color_col: Option<&str>) -> Result<()> {
    if let Some(color_col) = color_col
        && !headers.iter().any(|h| h == color_col)
    {
        let suffix = crate::diagnostics::format_column_suffix(
            crate::diagnostics::suggest_column(headers, color_col).as_deref(),
            color_col,
        );
        anyhow::bail!(
            "Color column '{}' not found. Available columns: {}{}",
            color_col,
            headers.join(", "),
            suffix
        );
    }
    Ok(())
}

/// Dispatch to the appropriate output renderer based on resolved params.
fn dispatch_output(
    params: &PipelineParams,
    recommendation: &ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
    y_opts: &recommend::YOptions,
    schema: &Schema,
) -> Result<()> {
    let query = &params.query;
    match params.output {
        Some(cli::OutputFormat::Table) => {
            let out_params = output::table::TableParams {
                chart_type_override: query.chart_type,
                agg: recommend::effective_agg(query, recommendation, schema),
                sort: params.sort,
                limit: params.limit,
                sort_flag: params.sort_flag,
            };
            output::table::print_table(recommendation, headers, rows, &out_params, schema)?;
        }
        Some(cli::OutputFormat::Spark) => {
            print_spark(recommendation, headers, rows, params, schema, y_opts);
        }
        Some(cli::OutputFormat::Svg) => {
            let opts = oneshot::RenderOptions::from_params(params, y_opts, recommendation, schema);
            output::svg::print_svg(recommendation, headers, rows, &opts)?;
        }
        Some(cli::OutputFormat::Html) => {
            let opts = oneshot::RenderOptions::from_params(params, y_opts, recommendation, schema);
            output::html::print_html(recommendation, headers, rows, &opts)?;
        }
        Some(cli::OutputFormat::Markdown) => {
            let out_params = output::table::TableParams {
                chart_type_override: query.chart_type,
                agg: recommend::effective_agg(query, recommendation, schema),
                sort: params.sort,
                limit: params.limit,
                sort_flag: params.sort_flag,
            };
            output::markdown::print_markdown(recommendation, headers, rows, &out_params, schema)?;
        }
        _ => {
            let opts = oneshot::RenderOptions::from_params(params, y_opts, recommendation, schema);
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
    params: &PipelineParams,
    schema: &Schema,
    y_opts: &recommend::YOptions,
) {
    let out_params = output::spark::SparkParams {
        chart_type_override: params.query.chart_type,
        agg: recommend::effective_agg(&params.query, recommendation, schema),
        sort: params.sort,
        limit: params.limit,
        color_col: params.query.color_col.clone(),
        bins: params.bins,
        extra_y_columns: y_opts
            .extra_columns
            .iter()
            .map(|(n, _)| n.clone())
            .collect(),
    };
    output::spark::print_spark(recommendation, headers, rows, &out_params);
}

/// Expand `--all-y`: add all remaining quantitative columns to extra_y.
fn expand_all_y(
    recommendation: &ChartRecommendation,
    schema: &Schema,
    y_opts: &mut recommend::YOptions,
) {
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
    params: &PipelineParams,
    y_opts: &recommend::YOptions,
) -> anyhow::Result<()> {
    let out_params = output::chart_json::ChartJsonParams {
        chart_type: params
            .query
            .chart_type
            .map(|ct| ct.to_chart_type())
            .unwrap_or(recommendation.chart_type),
        sort: params.sort,
        agg: recommend::effective_agg(&params.query, recommendation, schema),
        limit: params.limit,
        extra_y_columns: y_opts.extra_columns.clone(),
        color_column: params.query.color_col.clone(),
        bins: params.bins,
        filters: params.filters.clone(),
        sample: params.sample,
    };
    output::chart_json::print_chart_json(
        file,
        data,
        schema,
        recommendation,
        &data.headers,
        &data.rows,
        &out_params,
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
    fn render_data_params_take_query_not_cli() {
        use clap::Parser as _;
        // RED: pipeline must accept resolved params (no &Cli) and honor them.
        let cli =
            crate::cli::Cli::try_parse_from(["vz", "data.csv", "-x", "city", "-t", "bar"]).unwrap();
        let params = cli.to_pipeline_params();
        assert_eq!(params.query.x_col.as_deref(), Some("city"));
        let data = loaded(
            &["city", "revenue"],
            &[&["Tokyo", "100"], &["Osaka", "200"]],
        );
        let dir = std::env::temp_dir();
        assert!(render_data(&params, data, &dir.join("probe.csv")).is_ok());
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

    #[test]
    fn validate_color_column_ok_for_known_column() {
        let headers = vec!["city".to_string(), "revenue".to_string()];
        assert!(validate_color_column(&headers, Some("city")).is_ok());
        assert!(validate_color_column(&headers, None).is_ok());
    }

    #[test]
    fn validate_color_column_suggests_close_match() {
        let headers = vec!["city".to_string(), "revenue".to_string()];
        let err = validate_color_column(&headers, Some("ctiy")).unwrap_err();
        let msg = format!("{:#}", err);
        assert!(msg.contains("Did you mean 'city'?"), "{msg}");
        let err = validate_color_column(&headers, Some("City")).unwrap_err();
        let msg = format!("{:#}", err);
        assert!(msg.contains("case-sensitive"), "{msg}");
    }

    fn big_loaded(first_type_dates: bool) -> LoadedData {
        // 200 rows: half dates, half garbage — order decides which half the
        // old head-only sampler saw. Even sampling must infer identically.
        let mut rows: Vec<Vec<String>> = Vec::new();
        let mut date_rows: Vec<Vec<String>> = (0..100)
            .map(|i| vec!["2024-01-01".to_string(), i.to_string()])
            .collect();
        let mut junk_rows: Vec<Vec<String>> = (0..100)
            .map(|i| vec![format!("not-a-date-{i}"), i.to_string()])
            .collect();
        if first_type_dates {
            rows.append(&mut date_rows);
            rows.append(&mut junk_rows);
        } else {
            rows.append(&mut junk_rows);
            rows.append(&mut date_rows);
        }
        LoadedData {
            headers: vec!["d".to_string(), "v".to_string()],
            rows,
        }
    }

    #[test]
    fn infer_from_data_is_order_independent() {
        let fwd = infer_from_data(&big_loaded(true));
        let rev = infer_from_data(&big_loaded(false));
        assert_eq!(
            fwd.columns[0].data_type, rev.columns[0].data_type,
            "head ({:?}) vs tail ({:?}) order must not change inference",
            fwd.columns[0].data_type, rev.columns[0].data_type
        );
    }
}
