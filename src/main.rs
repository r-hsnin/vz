pub mod chart;
pub mod cli;
pub mod diagnostics;
pub mod explore;
pub mod filter;
pub mod infer;
pub mod loader;
pub mod oneshot;
pub mod output;
pub mod present;
pub mod render;
pub mod sparkline;
pub mod theme;
pub mod util;
pub mod watch;

use anyhow::Result;
use clap::{CommandFactory, Parser};

use std::path::{Path, PathBuf};

use cli::{Cli, Command, parse_column_spec, parse_multi_y_specs};
use infer::types::Schema;
use loader::LoadedData;

/// Infer schema from loaded data (eliminates boilerplate in multiple call sites).
fn infer_from_data(data: &LoadedData) -> infer::types::Schema {
    let headers: Vec<&str> = data.headers.iter().map(|s| s.as_str()).collect();
    let rows: Vec<Vec<&str>> = data
        .rows
        .iter()
        .map(|r| r.iter().map(|s| s.as_str()).collect())
        .collect();
    infer::infer_schema(&headers, &rows)
}

/// Print column metadata for --info flag.
fn print_info(file: &Path, data: &LoadedData, schema: &infer::types::Schema) {
    println!("File: {}", file.display());
    println!("Rows: {}", data.rows.len());
    println!("Columns: {}", schema.columns.len());
    println!();
    println!("{:<20} {:<15} {:>6}  Stats", "Name", "Type", "Nulls");
    println!("{}", "-".repeat(70));
    for (i, col) in schema.columns.iter().enumerate() {
        let stats = output::stats_text::compute_column_stats_text(i, &col.data_type, data);
        println!(
            "{:<20} {:<15} {:>6}  {}",
            col.name, col.data_type, col.null_count, stats
        );
    }
    println!();
    print_recommendation(schema);
}

/// Print column metadata as JSON for machine-readable output.
fn print_info_json(file: &Path, data: &LoadedData, schema: &Schema) -> anyhow::Result<()> {
    let recommendation = chart::select_chart(schema, None, None).ok();
    let output = output::build_info_output(
        &file.display().to_string(),
        data,
        schema,
        recommendation.as_ref(),
    );
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Print chart data as JSON — delegates to output::chart_json module.
fn print_chart_json(
    file: &Path,
    data: &loader::LoadedData,
    schema: &infer::types::Schema,
    recommendation: &chart::ChartRecommendation,
    cli: &Cli,
    y_opts: &YOptions,
) -> anyhow::Result<()> {
    let params = output::chart_json::ChartJsonParams {
        chart_type: cli
            .chart_type
            .map(|ct| ct.to_chart_type())
            .unwrap_or(recommendation.chart_type),
        sort: effective_sort(cli),
        agg: cli.agg.unwrap_or(cli::AggFunction::Sum),
        limit: cli.top.or(cli.tail),
        extra_y_columns: y_opts.extra_columns.clone(),
        color_column: cli.color_col.clone(),
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

/// Print the auto-detected chart recommendation for the data.
fn print_recommendation(schema: &Schema) {
    match chart::select_chart(schema, None, None) {
        Ok(rec) => {
            let y_part = rec
                .y_column
                .as_ref()
                .map(|y| format!(", y={}", y))
                .unwrap_or_default();
            let color_part = rec
                .color_column
                .as_ref()
                .map(|c| format!(", color={}", c))
                .unwrap_or_default();
            println!(
                "Recommendation: {} (x={}{}{})",
                rec.chart_type, rec.x_column, y_part, color_part
            );
        }
        Err(_) => {
            println!("Recommendation: (insufficient data for chart selection)");
        }
    }
}

/// Compute summary statistics for a column and format as human-readable text.
fn main() {
    let mut cli = Cli::parse();
    // --json is a shorthand for -o json
    if cli.json {
        cli.output = Some(cli::OutputFormat::Json);
    }
    // --spark is a shorthand for -o spark
    if cli.spark {
        cli.output = Some(cli::OutputFormat::Spark);
    }
    // --svg is a shorthand for -o svg
    if cli.svg {
        cli.output = Some(cli::OutputFormat::Svg);
    }
    // --markdown is a shorthand for -o markdown
    if cli.markdown {
        cli.output = Some(cli::OutputFormat::Markdown);
    }
    let json_output = cli.output == Some(cli::OutputFormat::Json);

    if let Err(e) = run(&cli) {
        if json_output {
            let err_json = serde_json::json!({
                "version": 1,
                "error": format!("{:#}", e),
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&err_json)
                    .unwrap_or_else(|_| { format!("{{\"version\":1,\"error\":\"{}\"}}", e) })
            );
            std::process::exit(1);
        } else {
            eprintln!("Error: {:#}", e);
            if let Some(hint) = diagnostics::error_hint(&e, &cli) {
                eprintln!("\n{}", hint);
            }
            std::process::exit(1);
        }
    }
}

fn run(cli: &Cli) -> Result<()> {
    match &cli.command {
        Some(Command::Explore { file, filter }) => {
            let data = loader::load_data(file)?;
            let data = apply_filters(data, filter)?;
            let schema = infer_from_data(&data);
            explore::run_explore(schema, data.rows, resolve_theme(cli))?;
        }
        Some(Command::Present { file }) => {
            present::run_present(file, resolve_theme(cli))?;
        }
        Some(Command::Completions { shell }) => {
            let mut cmd = Cli::command();
            clap_complete::generate(*shell, &mut cmd, "vz", &mut std::io::stdout());
        }
        None => run_oneshot(cli)?,
    }

    Ok(())
}

/// Print data as a formatted text table (used by `--output table`).
/// Print sparkline output (delegates to output::spark module).
fn print_spark(
    recommendation: &chart::selector::ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
    cli: &Cli,
) {
    let params = output::spark::SparkParams {
        chart_type_override: cli.chart_type,
        agg: cli.agg.unwrap_or(cli::AggFunction::Sum),
        sort: effective_sort(cli),
        limit: cli.top.or(cli.tail),
        color_col: cli.color_col.clone(),
    };
    output::spark::print_spark(recommendation, headers, rows, &params);
}

/// Expand `--all-y`: add all remaining quantitative columns to extra_y.
fn expand_all_y(
    recommendation: &chart::selector::ChartRecommendation,
    schema: &Schema,
    y_opts: &mut YOptions,
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

/// Run the oneshot (default) mode: load data, infer types, render chart.
fn run_oneshot(cli: &Cli) -> Result<()> {
    let file = resolve_input_file(cli)?;

    // If --watch is set, enter the file-watching loop
    if cli.watch {
        let cli_clone = cli.clone();
        return watch::run_watch(&file, || render_once(&cli_clone, &file));
    }

    render_once(cli, &file)
}

/// Single render pass: load → infer → render. Used by both normal and watch modes.
fn render_once(cli: &Cli, file: &Path) -> Result<()> {
    let data = loader::load_data_full(file, cli.no_header, format_override(cli))?;
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

    let schema = infer_from_data(&data);

    if cli.info {
        if cli.output == Some(cli::OutputFormat::Json) {
            print_info_json(file, &data, &schema)?;
        } else {
            print_info(file, &data, &schema);
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

    dispatch_output(cli, &recommendation, &data.headers, &data.rows, &y_opts)
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
    recommendation: &chart::ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
    y_opts: &YOptions,
) -> Result<()> {
    match cli.output {
        Some(cli::OutputFormat::Table) => {
            output::table::print_table(recommendation, headers, rows, cli)?;
        }
        Some(cli::OutputFormat::Spark) => {
            print_spark(recommendation, headers, rows, cli);
        }
        Some(cli::OutputFormat::Svg) => {
            let opts = build_render_options(cli, y_opts);
            print_svg(recommendation, headers, rows, &opts)?;
        }
        Some(cli::OutputFormat::Markdown) => {
            output::markdown::print_markdown(recommendation, headers, rows, cli)?;
        }
        _ => {
            let opts = build_render_options(cli, y_opts);
            oneshot::render_oneshot(recommendation, headers, rows, &opts)?;
        }
    }
    Ok(())
}

/// Render the chart to SVG and print to stdout.
fn print_svg(
    recommendation: &chart::ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
    opts: &oneshot::RenderOptions<'_>,
) -> anyhow::Result<()> {
    use ratatui::{buffer::Buffer, layout::Rect};

    let width = opts.width.unwrap_or_else(oneshot::terminal_width);
    let chart_type = oneshot::resolve_chart_type(recommendation, opts.chart_type_override);
    let height = opts.height.unwrap_or(24);

    let area = Rect::new(0, 0, width, height);
    let mut buf = Buffer::empty(area);
    oneshot::render_chart_to_buffer(
        chart_type,
        recommendation,
        headers,
        rows,
        opts,
        area,
        &mut buf,
    );

    println!(
        "{}",
        output::svg::buffer_to_svg(&buf, opts.theme.svg_background())
    );
    Ok(())
}

/// Construct render options from CLI args and parsed Y-column config.
fn build_render_options<'a>(cli: &'a Cli, y_opts: &'a YOptions) -> oneshot::RenderOptions<'a> {
    oneshot::RenderOptions {
        chart_type_override: cli.chart_type,
        y_label_override: y_opts.label_override.as_deref(),
        width: cli.width,
        height: cli.height,
        sort_order: effective_sort(cli),
        extra_y_columns: y_opts.extra_columns.clone(),
        limit: cli.top.or(cli.tail),
        agg: cli.agg.unwrap_or(cli::AggFunction::Sum),
        title: cli.title.clone(),
        labels: cli.labels,
        theme: resolve_theme(cli),
        bins: cli.bins,
    }
}

/// Resolve the theme from CLI args.
fn resolve_theme(cli: &Cli) -> theme::Theme {
    match cli.theme {
        Some(cli::ThemeArg::Light) => theme::Theme::light(),
        Some(cli::ThemeArg::HighContrast) => theme::Theme::high_contrast(),
        _ => theme::Theme::dark(),
    }
}

fn resolve_input_file(cli: &Cli) -> Result<PathBuf> {
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

fn build_recommendation(
    cli: &Cli,
    schema: &Schema,
    y_opts: &YOptions,
) -> Result<chart::selector::ChartRecommendation> {
    let x_hint = cli.x_col.as_deref().map(|s| parse_column_spec(s).0);
    let mut recommendation = chart::select_chart(schema, x_hint, y_opts.hint.as_deref())
        .map_err(|e| anyhow::anyhow!("{}", e))?;

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

/// Determine effective sort order: --top implies desc, --tail implies asc.
fn effective_sort(cli: &Cli) -> Option<cli::SortOrder> {
    if cli.sort.is_some() {
        return cli.sort;
    }
    if cli.top.is_some() {
        return Some(cli::SortOrder::Desc);
    }
    if cli.tail.is_some() {
        return Some(cli::SortOrder::Asc);
    }
    None
}

/// Convert CLI format argument to loader InputFormat.
fn format_override(cli: &Cli) -> Option<loader::InputFormat> {
    cli.format.map(|f| match f {
        cli::InputFormatArg::Csv => loader::InputFormat::Csv,
        cli::InputFormatArg::Tsv => loader::InputFormat::Tsv,
        cli::InputFormatArg::Json => loader::InputFormat::Json,
        cli::InputFormatArg::Ndjson => loader::InputFormat::Ndjson,
    })
}

/// Parse and apply --where filters to loaded data.
fn apply_filters(data: LoadedData, filters: &[String]) -> Result<LoadedData> {
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
struct YOptions {
    hint: Option<String>,
    label_override: Option<String>,
    extra_columns: Vec<(String, Option<String>)>,
}

/// Parse Y-axis options: primary Y hint, label override, and extra Y columns.
fn parse_y_options(cli: &Cli) -> YOptions {
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
/// Bar charts are most useful with categorical X (aggregation), not temporal X (per-row bars).
fn adjust_bar_recommendation(
    recommendation: &mut chart::ChartRecommendation,
    schema: &infer::types::Schema,
) {
    use infer::types::DataType;

    // Only adjust if current X is not already categorical
    let x_meta = schema
        .columns
        .iter()
        .find(|c| c.name == recommendation.x_column);
    if x_meta.map(|c| c.data_type) == Some(DataType::Categorical) {
        return;
    }

    // Find a categorical column to use as X
    let cat_cols = schema.columns_of_type(DataType::Categorical);
    if let Some(cat_col) = cat_cols.first() {
        recommendation.x_column = cat_col.name.clone();

        // Clear color column if it now matches X (redundant grouping)
        if recommendation.color_column.as_deref() == Some(cat_col.name.as_str()) {
            recommendation.color_column = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use infer::types::DataType;

    fn make_schema(cols: &[(&str, DataType)]) -> infer::types::Schema {
        infer::types::Schema::new(
            cols.iter()
                .map(|(name, dt)| infer::types::ColumnMeta {
                    name: name.to_string(),
                    data_type: *dt,
                    null_count: 0,
                    sample_size: 10,
                })
                .collect(),
        )
    }

    fn make_recommendation(
        x: &str,
        y: Option<&str>,
        color: Option<&str>,
    ) -> chart::ChartRecommendation {
        chart::ChartRecommendation {
            chart_type: chart::selector::ChartType::Bar,
            x_column: x.to_string(),
            y_column: y.map(|s| s.to_string()),
            color_column: color.map(|s| s.to_string()),
        }
    }

    #[test]
    fn test_adjust_bar_x_already_categorical_is_noop() {
        let schema = make_schema(&[
            ("city", DataType::Categorical),
            ("revenue", DataType::Quantitative),
        ]);
        let mut rec = make_recommendation("city", Some("revenue"), None);
        adjust_bar_recommendation(&mut rec, &schema);
        assert_eq!(rec.x_column, "city");
    }

    #[test]
    fn test_adjust_bar_no_categorical_cols_no_change() {
        let schema = make_schema(&[
            ("date", DataType::Temporal),
            ("revenue", DataType::Quantitative),
            ("profit", DataType::Quantitative),
        ]);
        let mut rec = make_recommendation("date", Some("revenue"), None);
        adjust_bar_recommendation(&mut rec, &schema);
        // No categorical column to swap to, so X stays
        assert_eq!(rec.x_column, "date");
    }

    #[test]
    fn test_adjust_bar_swaps_temporal_to_categorical() {
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
    fn test_adjust_bar_clears_color_when_matches_new_x() {
        let schema = make_schema(&[
            ("date", DataType::Temporal),
            ("city", DataType::Categorical),
            ("revenue", DataType::Quantitative),
        ]);
        // color=city, and city becomes the new X → color should be cleared
        let mut rec = make_recommendation("date", Some("revenue"), Some("city"));
        adjust_bar_recommendation(&mut rec, &schema);
        assert_eq!(rec.x_column, "city");
        assert_eq!(rec.color_column, None);
    }

    #[test]
    fn test_adjust_bar_preserves_color_when_different() {
        let schema = make_schema(&[
            ("date", DataType::Temporal),
            ("region", DataType::Categorical),
            ("city", DataType::Categorical),
            ("revenue", DataType::Quantitative),
        ]);
        // color=city, but region is first categorical → becomes X, color preserved
        let mut rec = make_recommendation("date", Some("revenue"), Some("city"));
        adjust_bar_recommendation(&mut rec, &schema);
        assert_eq!(rec.x_column, "region");
        assert_eq!(rec.color_column, Some("city".to_string()));
    }
}
