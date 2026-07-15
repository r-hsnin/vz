//! File discovery and filtering for directory mode.

use std::path::{Path, PathBuf};

use anyhow::Result;

/// Options controlling file discovery in a directory.
pub struct ScanOptions {
    /// Glob pattern to filter filenames (e.g. "sales_*.csv").
    pub glob_pattern: Option<String>,
}

/// A discovered data file entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    /// Full path to the file.
    pub path: PathBuf,
    /// Filename stem (without extension), used for `_source` column.
    pub stem: String,
}

/// Supported data file extensions.
const DATA_EXTENSIONS: &[&str] = &["csv", "tsv", "json", "ndjson", "jsonl", "tab"];

/// Scan a directory for data files.
///
/// Returns entries sorted lexicographically by filename.
/// Skips hidden files (starting with '.').
/// Only scans one level deep (no recursion).
#[allow(unused)]
pub fn scan_directory(dir: &Path, opts: &ScanOptions) -> Result<Vec<FileEntry>> {
    todo!()
}

/// Match a filename against a glob pattern supporting `*` and `?`.
#[allow(unused)]
pub fn glob_matches(pattern: &str, filename: &str) -> bool {
    todo!()
}
