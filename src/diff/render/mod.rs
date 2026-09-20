//! Diff-aware rendering: dispatches to format-specific sub-modules.

mod bar;
mod html;
mod json;
mod line;
mod markdown;
mod spark;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use anyhow::Result;
use std::path::Path;

use crate::chart::selector::SortOrder;
use crate::cli::{self, DiffParams};
use crate::diff::{DiffEntry, DiffResult, DiffTimeSeries};

/// Render the diff result based on CLI output format.
pub(crate) fn render_diff(
    params: &DiffParams,
    diff: &DiffResult,
    before_path: &Path,
    after_path: &Path,
) -> Result<()> {
    match params.output {
        Some(cli::OutputFormat::Spark) => {
            spark::print_diff_spark(diff);
        }
        Some(cli::OutputFormat::Json) => {
            json::print_diff_json(diff, before_path, after_path)?;
        }
        Some(cli::OutputFormat::Markdown) => {
            markdown::print_diff_markdown(params, diff, before_path, after_path);
        }
        Some(cli::OutputFormat::Html) => {
            html::print_diff_html(params, diff, before_path, after_path);
        }
        _ => {
            bar::print_diff_summary(diff, before_path, after_path);
            bar::print_diff_bar(params, diff);
        }
    }
    Ok(())
}

/// Render temporal diff as a 2-series line chart overlay.
pub(crate) fn render_diff_line(
    params: &DiffParams,
    ts: &DiffTimeSeries,
    before_path: &Path,
    after_path: &Path,
) -> Result<()> {
    match params.output {
        Some(cli::OutputFormat::Spark) => {
            spark::print_diff_line_spark(ts, before_path, after_path);
        }
        Some(cli::OutputFormat::Json) => {
            json::print_diff_line_json(ts, before_path, after_path)?;
        }
        Some(cli::OutputFormat::Markdown) => {
            markdown::print_diff_line_markdown(ts, before_path, after_path);
        }
        Some(cli::OutputFormat::Html) => {
            html::print_diff_line_html(params, ts, before_path, after_path);
        }
        _ => {
            line::print_diff_line_summary(ts, before_path, after_path);
            line::print_diff_line_chart(params, ts, before_path, after_path)?;
        }
    }
    Ok(())
}

/// Apply sort and limit (--top, --tail, --sort) to diff entries.
pub(super) fn apply_sort_and_limit(
    sort: Option<SortOrder>,
    limit: Option<usize>,
    entries: &[DiffEntry],
) -> Vec<DiffEntry> {
    let mut sorted = entries.to_vec();

    match sort {
        Some(SortOrder::Desc) => {
            sorted.sort_by(|a, b| {
                b.delta
                    .partial_cmp(&a.delta)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        Some(SortOrder::Asc) => {
            sorted.sort_by(|a, b| {
                a.delta
                    .partial_cmp(&b.delta)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        _ => {} // preserve original order
    }

    if let Some(n) = limit {
        sorted.truncate(n);
    }

    sorted
}
