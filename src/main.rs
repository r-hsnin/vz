pub mod chart;
pub mod cli;
pub mod diagnostics;
pub mod diff;
pub mod directory;
pub mod explore;
pub mod filter;
mod helpers;
pub mod infer;
mod info;
pub mod loader;
pub mod oneshot;
pub mod output;
mod pipeline;
pub mod present;
pub mod render;
pub mod sparkline;
pub mod theme;
pub mod util;
pub mod watch;

use anyhow::Result;
use clap::{CommandFactory, Parser};

use std::path::Path;

use cli::{Cli, Command};
use helpers::{apply_filters, format_override, resolve_input_file, resolve_theme};

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
    // --html is a shorthand for -o html
    if cli.html {
        cli.output = Some(cli::OutputFormat::Html);
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
            let data = if file.is_dir() {
                let opts = directory::scanner::ScanOptions {
                    glob_pattern: None,
                    recurse: false,
                };
                let entries = directory::scanner::scan_directory(file, &opts)?;
                let result = directory::combiner::combine_files(&entries, false)?;
                result.data
            } else {
                loader::load_data(file)?
            };
            let data = apply_filters(data, filter)?;
            let schema = pipeline::infer_from_data(&data);
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

/// Run the oneshot (default) mode: load data, infer types, render chart.
fn run_oneshot(cli: &Cli) -> Result<()> {
    // Diff mode: two files provided
    if let Some((before, after)) = cli.diff_pair() {
        return diff::run_diff(cli, &before, &after);
    }

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
    if file.is_dir() {
        return directory::run_directory(cli, file);
    }

    if cli.catalog {
        anyhow::bail!("--catalog requires a directory argument, not a file");
    }

    if cli.bins == Some(0) {
        anyhow::bail!("--bins must be at least 1");
    }
    if cli.top == Some(0) {
        anyhow::bail!("--top must be at least 1");
    }
    if cli.tail == Some(0) {
        anyhow::bail!("--tail must be at least 1");
    }

    let data = loader::load_data_full(file, cli.no_header, format_override(cli))?;
    pipeline::render_data(cli, data, file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::adjust_bar_recommendation;
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
