//! Binary dispatch: CLI parsing outcomes → mode execution.
//!
//! Owns `Cli` end-to-end. The library exposes only [`run`] (taking the
//! already-parsed [`Cli`](crate::cli::Cli)); `main.rs` stays a thin
//! shorthand-normalization + error-reporting wrapper.

use anyhow::Result;
use clap::CommandFactory;
use std::path::Path;

use crate::cli::{Cli, Command, format_override, resolve_input_file, resolve_theme};
use crate::diagnostics;
use crate::directory;
use crate::filter::apply_filters;
use crate::loader;
use crate::pipeline;
use crate::{diff, explore, present};

/// Normalize `--json/--spark/--svg/--markdown/--html` shorthands into `output`.
pub fn apply_output_shorthands(cli: &mut Cli) {
    use crate::cli::OutputFormat;
    if cli.json {
        cli.output = Some(OutputFormat::Json);
    }
    if cli.spark {
        cli.output = Some(OutputFormat::Spark);
    }
    if cli.svg {
        cli.output = Some(OutputFormat::Svg);
    }
    if cli.markdown {
        cli.output = Some(OutputFormat::Markdown);
    }
    if cli.html {
        cli.output = Some(OutputFormat::Html);
    }
}

/// Dispatch a parsed CLI invocation. `json_errors` selects JSON error
/// reporting (set when the resolved output format is JSON).
pub fn run(cli: &Cli, json_errors: bool) -> Result<()> {
    if let Err(e) = dispatch(cli) {
        if json_errors {
            let err_json = serde_json::json!({
                "version": 1,
                "error": format!("{:#}", e),
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&err_json)
                    .unwrap_or_else(|_| { format!("{{\"version\":1,\"error\":\"{}\"}}", e) })
            );
        } else {
            eprintln!("Error: {:#}", e);
            if let Some(hint) = diagnostics::error_hint(&e, cli.primary_file()) {
                eprintln!("\n{}", hint);
            }
        }
        std::process::exit(1);
    }
    Ok(())
}

fn dispatch(cli: &Cli) -> Result<()> {
    match &cli.command {
        Some(Command::Explore { file, filter }) => {
            if file.len() == 2 {
                run_explore_diff(&file[0], &file[1], cli)?;
            } else {
                let path = &file[0];
                let data = if path.is_dir() {
                    let opts = directory::scanner::ScanOptions {
                        glob_pattern: None,
                        recurse: false,
                    };
                    let entries = directory::scanner::scan_directory(path, &opts)?;
                    let result = directory::combiner::combine_files(&entries, false)?;
                    result.data
                } else {
                    loader::load_data(path)?
                };
                let outcome = apply_filters(data, filter)?;
                if let Some(notice) = outcome.notice {
                    eprintln!("{notice}");
                }
                let data = outcome.data;
                let schema = pipeline::infer_from_data(&data);
                explore::run_explore(schema, data.rows, resolve_theme(cli))?;
            }
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

/// Run explore mode with two files (diff exploration).
fn run_explore_diff(before_path: &Path, after_path: &Path, cli: &Cli) -> Result<()> {
    let before = loader::load_data(before_path)?;
    let after = loader::load_data(after_path)?;

    diff::validate_schema(&before, &after, before_path, after_path)?;

    let schema = pipeline::infer_from_data(&before);
    let theme = resolve_theme(cli);

    // Resolve X column: first categorical or temporal
    let x_col = diff::auto_x_column(&schema, &before.headers)
        .unwrap_or_else(|| before.headers.first().cloned().unwrap_or_default());

    // Resolve Y column: first quantitative that is not X
    let y_col = diff::auto_y_column(&schema, &x_col)
        .ok_or_else(|| anyhow::anyhow!("No quantitative column found for Y axis"))?;

    // Detect temporal X
    let x_is_temporal = diff::is_temporal_column(&schema, &x_col);

    let diff_data = if x_is_temporal {
        explore::DiffData::Temporal(diff::compute_diff_temporal(
            &before, &after, &x_col, &y_col,
        )?)
    } else {
        explore::DiffData::Categorical(diff::compute_diff(&before, &after, &x_col, &y_col)?)
    };

    let before_name = before_path
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_else(|| "before".into());
    let after_name = after_path
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_else(|| "after".into());

    explore::run_explore_diff(diff_data, before_name, after_name, theme)
}

/// Run the oneshot (default) mode: load data, infer types, render chart.
fn run_oneshot(cli: &Cli) -> Result<()> {
    // Diff mode: two files provided
    if let Some((before, after)) = cli.diff_pair() {
        return diff::run_diff_from_cli(cli, &before, &after);
    }

    let file = resolve_input_file(cli)?;

    // If --watch is set, enter the file-watching loop
    if cli.watch {
        let cli_clone = cli.clone();
        return crate::watch::run_watch(&file, || render_once(&cli_clone, &file));
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
    pipeline::render_data_from_cli(cli, data, file)
}
