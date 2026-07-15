//! Directory mode: combine multiple data files from a directory.
//!
//! Scans a directory for data files, validates schema compatibility,
//! concatenates rows, and injects a `_source` column with each file's stem.

#[cfg(test)]
mod auto_sample_tests;
pub mod catalog;
#[cfg(test)]
mod catalog_tests;
pub mod combiner;
pub mod date_extract;
pub mod scanner;
#[cfg(test)]
mod tests;

use std::path::Path;

use anyhow::Result;

use crate::cli::Cli;

use self::combiner::combine_files;
use self::scanner::{ScanOptions, scan_directory};

/// Row count threshold above which a large dataset warning is emitted.
pub const LARGE_DATASET_THRESHOLD: usize = 100_000;

/// Maximum combined rows before auto-sampling kicks in for directory mode.
pub const MAX_COMBINED_ROWS: usize = 1_000_000;

/// Returns a warning message if the row count exceeds the threshold.
pub fn large_dataset_warning(row_count: usize) -> Option<String> {
    if row_count > LARGE_DATASET_THRESHOLD {
        Some(format!(
            "warning: large dataset ({row_count} rows). Consider --sample for faster rendering."
        ))
    } else {
        None
    }
}

/// Auto-sample combined data if it exceeds the row limit.
///
/// Returns the (possibly sampled) data and an optional warning message.
/// Uses systematic (every-Nth) sampling to preserve distribution.
/// If `no_limit` is true, bypasses sampling entirely.
pub fn auto_sample_combined(
    mut data: crate::loader::LoadedData,
    max_rows: usize,
    no_limit: bool,
) -> (crate::loader::LoadedData, Option<String>) {
    if no_limit || data.rows.len() <= max_rows {
        return (data, None);
    }

    let total = data.rows.len();
    let step = total as f64 / max_rows as f64;
    let sampled: Vec<Vec<String>> = (0..max_rows)
        .map(|i| {
            let idx = (i as f64 * step).floor() as usize;
            data.rows[idx.min(total - 1)].clone()
        })
        .collect();

    let warning = format!(
        "warning: dataset exceeded {} rows, auto-sampled to {} rows",
        max_rows,
        sampled.len()
    );
    data.rows = sampled;
    (data, Some(warning))
}

/// Run directory mode: scan, combine, and render data from a directory.
pub fn run_directory(cli: &Cli, dir: &Path) -> Result<()> {
    let opts = ScanOptions {
        glob_pattern: cli.glob.clone(),
        recurse: cli.recurse,
    };

    let entries = scan_directory(dir, &opts)?;

    // Catalog mode: show schema inventory without combining
    if cli.catalog {
        return catalog::run_catalog(cli, &entries);
    }

    let result = combine_files(&entries, cli.no_header)?;

    // Auto-sample if row count exceeds limit (unless --no-limit)
    let (data, auto_sample_warning) =
        auto_sample_combined(result.data, MAX_COMBINED_ROWS, cli.no_limit);

    // Emit large dataset warning only if auto-sampling did NOT fire
    if auto_sample_warning.is_none() {
        if let Some(warning) = large_dataset_warning(data.rows.len()) {
            eprintln!("{warning}");
        }
    }

    // Print summary to stderr
    let summary = if result.skipped.is_empty() {
        format!("{} files, {} rows", result.file_count, data.rows.len())
    } else {
        format!(
            "{} files, {} rows ({} skipped)",
            result.file_count,
            data.rows.len(),
            result.skipped.len()
        )
    };
    eprintln!("info: {summary}");

    // Print auto-sampling warning after summary
    if let Some(ref warning) = auto_sample_warning {
        eprintln!("{warning}");
    }

    // Print skip warnings to stderr
    for skip in &result.skipped {
        eprintln!("warning: skipped '{}': {}", skip.file, skip.reason);
    }

    // Feed combined data into the standard render pipeline
    crate::render_data(cli, data, dir)
}
