//! Directory mode: combine multiple data files from a directory.
//!
//! Scans a directory for data files, validates schema compatibility,
//! concatenates rows, and injects a `_source` column with each file's stem.

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

/// Run directory mode: scan, combine, and render data from a directory.
pub fn run_directory(cli: &Cli, dir: &Path) -> Result<()> {
    let opts = ScanOptions {
        glob_pattern: cli.glob.clone(),
        recurse: cli.recurse,
    };

    let entries = scan_directory(dir, &opts)?;
    let result = combine_files(&entries, cli.no_header)?;

    // Emit large dataset warning if needed
    if let Some(warning) = large_dataset_warning(result.data.rows.len()) {
        eprintln!("{warning}");
    }

    // Print summary to stderr
    let summary = if result.skipped.is_empty() {
        format!(
            "{} files, {} rows",
            result.file_count,
            result.data.rows.len()
        )
    } else {
        format!(
            "{} files, {} rows ({} skipped)",
            result.file_count,
            result.data.rows.len(),
            result.skipped.len()
        )
    };
    eprintln!("info: {summary}");

    // Print skip warnings to stderr
    for skip in &result.skipped {
        eprintln!("warning: skipped '{}': {}", skip.file, skip.reason);
    }

    // Feed combined data into the standard render pipeline
    crate::render_data(cli, result.data, dir)
}
