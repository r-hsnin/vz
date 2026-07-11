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

use anyhow::Result;
use clap::Parser;

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
    let cli = Cli::parse();
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
            explore::run_explore(schema, data.rows)?;
        }
        Some(Command::Present { file }) => {
            present::run_present(file)?;
        }
        None => run_oneshot(cli)?,
    }

    Ok(())
}

/// Run the oneshot (default) mode: load data, infer types, render chart.
fn run_oneshot(cli: &Cli) -> Result<()> {
    let file = resolve_input_file(cli)?;
    let data = loader::load_data_full(&file, cli.no_header, format_override(cli))?;
    let data = apply_filters(data, &cli.filter)?;
    let data = if let Some(max_rows) = cli.sample {
        loader::apply_sampling(data, max_rows)
    } else {
        data
    };

    if data.rows.is_empty() {
        anyhow::bail!(
            "No data rows found in '{}'. The file appears to contain only headers.",
            file.display(),
        );
    }

    let schema = infer_from_data(&data);

    if cli.info {
        if cli.output == Some(cli::OutputFormat::Json) {
            print_info_json(&file, &data, &schema)?;
        } else {
            print_info(&file, &data, &schema);
        }
        return Ok(());
    }

    // JSON output without --info: output chart metadata + data summary
    if cli.output == Some(cli::OutputFormat::Json) {
        print_info_json(&file, &data, &schema)?;
        return Ok(());
    }

    let mut y_opts = parse_y_options(cli);
    let recommendation = build_recommendation(cli, &schema, &y_opts)?;

    // --all-y: overlay all quantitative columns as multi-series
    if cli.all_y {
        let x_col = &recommendation.x_column;
        let primary_y = recommendation.y_column.as_deref().unwrap_or("");
        let extra: Vec<(String, Option<String>)> = schema
            .columns
            .iter()
            .filter(|c| c.data_type == infer::types::DataType::Quantitative)
            .filter(|c| c.name != *x_col && c.name != primary_y)
            .filter(|c| {
                // Skip if already in extra_y_columns
                !y_opts.extra_columns.iter().any(|(n, _)| n == &c.name)
            })
            .map(|c| (c.name.clone(), None))
            .collect();
        y_opts.extra_columns.extend(extra);
    }

    oneshot::render_oneshot(
        &recommendation,
        &data.headers,
        &data.rows,
        &oneshot::RenderOptions {
            chart_type_override: cli.chart_type.as_deref(),
            y_label_override: y_opts.label_override.as_deref(),
            width: cli.width,
            height: cli.height,
            sort_order: effective_sort(cli),
            extra_y_columns: y_opts.extra_columns,
            limit: cli.top.or(cli.tail),
            agg: cli.agg.unwrap_or(cli::AggFunction::Sum),
            title: cli.title.clone(),
            labels: cli.labels,
        },
    )?;

    Ok(())
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

    if cli.chart_type.as_deref() == Some("bar") && cli.x_col.is_none() {
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
}
