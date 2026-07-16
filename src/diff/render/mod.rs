//! Diff-aware rendering: dispatches to format-specific sub-modules.

mod bar;
mod json;
mod line;
mod spark;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use anyhow::Result;
use std::path::Path;

use crate::cli::{self, Cli};
use crate::diff::{DiffEntry, DiffResult, DiffTimeSeries};

/// Render the diff result based on CLI output format.
pub fn render_diff(
    cli: &Cli,
    diff: &DiffResult,
    before_path: &Path,
    after_path: &Path,
) -> Result<()> {
    match cli.output {
        Some(cli::OutputFormat::Spark) => {
            spark::print_diff_spark(diff);
        }
        Some(cli::OutputFormat::Json) => {
            json::print_diff_json(diff, before_path, after_path)?;
        }
        _ => {
            bar::print_diff_summary(diff, before_path, after_path);
            bar::print_diff_bar(cli, diff);
        }
    }
    Ok(())
}

/// Render temporal diff as a 2-series line chart overlay.
pub fn render_diff_line(
    cli: &Cli,
    ts: &DiffTimeSeries,
    before_path: &Path,
    after_path: &Path,
) -> Result<()> {
    match cli.output {
        Some(cli::OutputFormat::Spark) => {
            spark::print_diff_line_spark(ts, before_path, after_path);
        }
        Some(cli::OutputFormat::Json) => {
            json::print_diff_line_json(ts, before_path, after_path)?;
        }
        _ => {
            line::print_diff_line_summary(ts, before_path, after_path);
            line::print_diff_line_chart(cli, ts, before_path, after_path)?;
        }
    }
    Ok(())
}

/// Apply sort and limit (--top, --tail, --sort) to diff entries.
pub(super) fn apply_sort_and_limit(cli: &Cli, entries: &[DiffEntry]) -> Vec<DiffEntry> {
    let mut sorted = entries.to_vec();

    match cli.effective_sort() {
        Some(cli::SortOrder::Desc) => {
            sorted.sort_by(|a, b| {
                b.delta
                    .partial_cmp(&a.delta)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        Some(cli::SortOrder::Asc) => {
            sorted.sort_by(|a, b| {
                a.delta
                    .partial_cmp(&b.delta)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        _ => {} // preserve original order
    }

    if let Some(n) = cli.top.or(cli.tail) {
        sorted.truncate(n);
    }

    sorted
}
