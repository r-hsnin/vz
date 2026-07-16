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
