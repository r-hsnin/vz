pub mod chart;
pub mod cli;
pub mod explore;
pub mod filter;
pub mod infer;
pub mod loader;
pub mod oneshot;
pub mod output;
pub mod present;
pub mod render;
pub mod sparkline;
pub mod svg;
pub mod table;
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
        let stats = compute_column_stats_text(i, &col.data_type, data);
        println!(
            "{:<20} {:<15} {:>6}  {}",
            col.name,
            format!("{:?}", col.data_type),
            col.null_count,
            stats
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

/// Print chart data as JSON (includes metadata for backward compat + chart_data field).
#[allow(clippy::too_many_arguments)]
fn print_chart_json(
    file: &Path,
    data: &loader::LoadedData,
    schema: &infer::types::Schema,
    recommendation: &chart::ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
    cli: &Cli,
    y_opts: &YOptions,
) -> anyhow::Result<()> {
    use chart::data_builder;
    use chart::selector::ChartType;
    use serde_json::json;

    // Build base info output (backward compatible)
    let output = output::build_info_output(
        &file.display().to_string(),
        data,
        schema,
        Some(recommendation),
    );

    // Compute chart_data from aggregation/sort
    let chart_type = cli
        .chart_type
        .map(|ct| ct.to_chart_type())
        .unwrap_or(recommendation.chart_type);
    let sort = effective_sort(cli);
    let agg = cli.agg.unwrap_or(cli::AggFunction::Sum);
    let limit = cli.top.or(cli.tail);

    let x_idx = data_builder::column_index(headers, &recommendation.x_column).unwrap_or(0);
    let y_idx = recommendation
        .y_column
        .as_ref()
        .and_then(|y| data_builder::column_index(headers, y))
        .unwrap_or(x_idx);

    let chart_data = match chart_type {
        ChartType::Bar => {
            let (mut bar_data, _) =
                data_builder::aggregate_bar(rows, x_idx, y_idx, None, String::new(), agg);
            oneshot::builders::sort_bar_data(&mut bar_data, sort);
            if let Some(n) = limit {
                bar_data.labels.truncate(n);
                bar_data.values.truncate(n);
            }
            json!({ "type": "bar", "categories": bar_data.labels, "values": bar_data.values })
        }
        ChartType::Histogram => {
            let hist_data = data_builder::build_histogram(rows, x_idx, None, String::new());
            let bins = render::compute_bins(&hist_data.values, hist_data.bin_count);
            let bin_data: Vec<serde_json::Value> = bins
                .iter()
                .map(|(s, e, c)| json!({"range": format!("{:.0}-{:.0}", s, e), "count": c}))
                .collect();
            json!({ "type": "histogram", "bins": bin_data })
        }
        _ => {
            let extra_y: Vec<usize> = y_opts
                .extra_columns
                .iter()
                .filter_map(|(name, _)| headers.iter().position(|h| h == name))
                .collect();
            let mut series: Vec<serde_json::Value> = Vec::new();
            let y_name = headers.get(y_idx).cloned().unwrap_or_default();
            let points: Vec<serde_json::Value> = rows
                .iter()
                .filter_map(|r| {
                    let x = r.get(x_idx)?.clone();
                    let y: f64 = r.get(y_idx)?.replace(',', "").parse().ok()?;
                    Some(json!({"x": x, "y": y}))
                })
                .collect();
            series.push(json!({"name": y_name, "data": points}));
            for &ey in &extra_y {
                let name = headers.get(ey).cloned().unwrap_or_default();
                let pts: Vec<serde_json::Value> = rows
                    .iter()
                    .filter_map(|r| {
                        let x = r.get(x_idx)?.clone();
                        let y: f64 = r.get(ey)?.replace(',', "").parse().ok()?;
                        Some(json!({"x": x, "y": y}))
                    })
                    .collect();
                series.push(json!({"name": name, "data": pts}));
            }
            json!({ "type": format!("{chart_type:?}").to_lowercase(), "series": series })
        }
    };

    // Merge chart_data into the output
    let mut output_value = serde_json::to_value(&output)?;
    if let serde_json::Value::Object(ref mut map) = output_value {
        map.insert("chart_data".to_string(), chart_data);
    }

    println!("{}", serde_json::to_string_pretty(&output_value)?);
    Ok(())
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
                "Recommendation: {:?} (x={}{}{})",
                rec.chart_type, rec.x_column, y_part, color_part
            );
        }
        Err(_) => {
            println!("Recommendation: (insufficient data for chart selection)");
        }
    }
}

/// Compute summary statistics for a column and format as human-readable text.
fn compute_column_stats_text(
    col_idx: usize,
    data_type: &infer::types::DataType,
    data: &LoadedData,
) -> String {
    use output::ColumnStats;
    match output::compute_column_stats(col_idx, data_type, data) {
        ColumnStats::Quantitative { min, max, mean } => {
            format!(
                "Min={}  Max={}  Mean={}",
                format_stat(min),
                format_stat(max),
                format_stat(mean)
            )
        }
        ColumnStats::Categorical { unique, .. } => format!("{} unique", unique),
        ColumnStats::Temporal { min, max } => {
            if min == max {
                min
            } else {
                format!("{}..{}", min, max)
            }
        }
        ColumnStats::Empty {} => String::new(),
    }
}

/// Format a numeric stat value concisely.
fn format_stat(val: f64) -> String {
    if val == val.trunc() && val.abs() < 1_000_000.0 {
        format!("{:.0}", val)
    } else if val.abs() >= 1_000_000.0 {
        format!("{:.2e}", val)
    } else {
        format!("{:.2}", val)
    }
}

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
            if let Some(hint) = error_hint(&e, &cli) {
                eprintln!("\n{}", hint);
            }
            std::process::exit(1);
        }
    }
}

/// Generate contextual hints for common errors.
fn error_hint(err: &anyhow::Error, cli: &Cli) -> Option<String> {
    let msg = format!("{:#}", err);
    // File not found: suggest similar files in the same directory
    if msg.contains("No such file")
        && let Some(ref file) = cli.file
    {
        let parent = file.parent().unwrap_or(std::path::Path::new("."));
        let stem = file.file_name()?.to_str()?;
        let suggestions = find_similar_files(parent, stem);
        if !suggestions.is_empty() {
            let list = suggestions
                .iter()
                .map(|s| format!("    • {}", s))
                .collect::<Vec<_>>()
                .join("\n");
            return Some(format!("  Did you mean?\n{}", list));
        }
        return Some("  Tip: use vz - to read from stdin".to_string());
    }
    // Empty data
    if msg.contains("No data rows") {
        return Some(
            "  Tip: check that the file contains data rows below the header.\n  \
             For headerless data, try: vz file.csv --no-header"
                .to_string(),
        );
    }
    None
}

/// Find files in `dir` with names similar to `target`.
/// Supported data file extensions for discovery.
const DATA_EXTENSIONS: &[&str] = &["csv", "tsv", "json", "ndjson", "jsonl", "tab"];

/// Check if a filename has a recognized data extension.
fn is_data_file(name: &str) -> bool {
    name.rsplit('.')
        .next()
        .is_some_and(|ext| DATA_EXTENSIONS.contains(&ext))
}

fn find_similar_files(dir: &std::path::Path, target: &str) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return vec![];
    };
    let target_lower = target.to_lowercase();
    let all_data_files: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().ok().is_some_and(|t| t.is_file()))
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|name| is_data_file(name))
        .collect();

    // First: find files similar to the target name
    let similar: Vec<String> = all_data_files
        .iter()
        .filter(|name| {
            let name_lower = name.to_lowercase();
            let shared = target_lower
                .chars()
                .zip(name_lower.chars())
                .take_while(|(a, b)| a == b)
                .count();
            shared >= 3 || name_lower.contains(&target_lower[..target_lower.len().min(4)])
        })
        .take(3)
        .cloned()
        .collect();

    if !similar.is_empty() {
        return similar;
    }
    // Fallback: show any data files in the directory
    all_data_files.into_iter().take(3).collect()
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
/// Generate a sparkline string from values, mapping to 8 Unicode block characters.
fn make_sparkline(values: &[f64]) -> String {
    sparkline::sparkline_from_values(values)
}

/// Print sparkline output: single-line or grouped by color column.
fn print_spark(
    recommendation: &chart::selector::ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
    cli: &Cli,
) {
    let y_idx = recommendation
        .y_column
        .as_ref()
        .and_then(|y| chart::data_builder::column_index(headers, y));
    let Some(yi) = y_idx else {
        println!("▄");
        return;
    };

    // If color column specified, output one sparkline per group
    if let Some(ref color) = cli.color_col
        && let Some(ci) = chart::data_builder::column_index(headers, color)
    {
        let mut groups: std::collections::BTreeMap<&str, Vec<f64>> =
            std::collections::BTreeMap::new();
        for row in rows {
            let group = row.get(ci).map_or("", |v| v.as_str());
            let val = row.get(yi).and_then(|v| v.parse::<f64>().ok());
            if let Some(v) = val {
                groups.entry(group).or_default().push(v);
            }
        }
        for (name, values) in &groups {
            println!("{}  {}", name, make_sparkline(values));
        }
        return;
    }

    // Single sparkline from all Y values in row order
    let values: Vec<f64> = rows
        .iter()
        .filter_map(|r| r.get(yi)?.parse::<f64>().ok())
        .collect();
    println!("{}", make_sparkline(&values));
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
        loader::apply_sampling(data, max_rows)
    } else {
        data
    };

    if data.rows.is_empty() {
        if !cli.filter.is_empty() && pre_filter_count > 0 {
            anyhow::bail!(
                "No rows remain after filtering. All {} rows were excluded by --where predicates.",
                pre_filter_count,
            );
        }
        anyhow::bail!(
            "No data rows found in '{}'. The file appears to contain only headers.",
            file.display(),
        );
    }

    let schema = infer_from_data(&data);

    if cli.info {
        if cli.output == Some(cli::OutputFormat::Json) {
            print_info_json(file, &data, &schema)?;
        } else {
            print_info(file, &data, &schema);
        }
        return Ok(());
    }

    // JSON output without --info: output metadata + chart data
    if cli.output == Some(cli::OutputFormat::Json) {
        let mut y_opts = parse_y_options(cli);
        let recommendation = build_recommendation(cli, &schema, &y_opts)?;
        if cli.all_y {
            expand_all_y(&recommendation, &schema, &mut y_opts);
        }
        print_chart_json(
            file,
            &data,
            &schema,
            &recommendation,
            &data.headers,
            &data.rows,
            cli,
            &y_opts,
        )?;
        return Ok(());
    }

    let mut y_opts = parse_y_options(cli);
    let recommendation = build_recommendation(cli, &schema, &y_opts)?;

    // --all-y: overlay all quantitative columns as multi-series
    if cli.all_y {
        expand_all_y(&recommendation, &schema, &mut y_opts);
    }

    dispatch_output(cli, &recommendation, &data.headers, &data.rows, &y_opts)
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
            table::print_table(recommendation, headers, rows, cli)?;
        }
        Some(cli::OutputFormat::Spark) => {
            print_spark(recommendation, headers, rows, cli);
        }
        Some(cli::OutputFormat::Svg) => {
            let opts = build_render_options(cli, y_opts);
            print_svg(recommendation, headers, rows, &opts)?;
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

    println!("{}", svg::buffer_to_svg(&buf));
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

    fn make_data(headers: Vec<&str>, rows: Vec<Vec<&str>>) -> LoadedData {
        LoadedData {
            headers: headers.into_iter().map(|s| s.to_string()).collect(),
            rows: rows
                .into_iter()
                .map(|r| r.into_iter().map(|s| s.to_string()).collect())
                .collect(),
        }
    }

    #[test]
    fn test_compute_column_stats_quantitative() {
        let data = make_data(vec!["val"], vec![vec!["10"], vec!["20"], vec!["30"]]);
        let result = compute_column_stats_text(0, &DataType::Quantitative, &data);
        assert!(result.contains("Min=10"));
        assert!(result.contains("Max=30"));
        assert!(result.contains("Mean=20"));
    }

    #[test]
    fn test_compute_column_stats_empty() {
        let data = make_data(vec!["val"], vec![vec![""], vec![""]]);
        let result = compute_column_stats_text(0, &DataType::Quantitative, &data);
        assert_eq!(result, "");
    }

    #[test]
    fn test_compute_column_stats_categorical() {
        let data = make_data(
            vec!["city"],
            vec![vec!["Tokyo"], vec!["Osaka"], vec!["Tokyo"]],
        );
        let result = compute_column_stats_text(0, &DataType::Categorical, &data);
        assert_eq!(result, "2 unique");
    }

    #[test]
    fn test_compute_column_stats_temporal() {
        let data = make_data(
            vec!["date"],
            vec![vec!["2024-01"], vec!["2024-02"], vec!["2024-03"]],
        );
        let result = compute_column_stats_text(0, &DataType::Temporal, &data);
        assert_eq!(result, "2024-01..2024-03");
    }

    #[test]
    fn test_compute_column_stats_temporal_single() {
        let data = make_data(vec!["date"], vec![vec!["2024-01"]]);
        let result = compute_column_stats_text(0, &DataType::Temporal, &data);
        assert_eq!(result, "2024-01");
    }

    #[test]
    fn test_compute_column_stats_non_numeric_quantitative() {
        let data = make_data(vec!["val"], vec![vec!["abc"], vec!["def"]]);
        let result = compute_column_stats_text(0, &DataType::Quantitative, &data);
        assert_eq!(result, "");
    }

    #[test]
    fn test_format_stat_integer() {
        assert_eq!(format_stat(42.0), "42");
        assert_eq!(format_stat(0.0), "0");
        assert_eq!(format_stat(-5.0), "-5");
    }

    #[test]
    fn test_format_stat_decimal() {
        assert_eq!(format_stat(3.75), "3.75");
        assert_eq!(format_stat(0.5), "0.50");
    }

    #[test]
    fn test_format_stat_large() {
        let result = format_stat(1_500_000.0);
        assert!(result.contains("e") || result.contains("E"));
    }

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
