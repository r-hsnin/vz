//! Schema matching and file combining for directory mode.

use anyhow::Result;

use crate::loader::LoadedData;

use super::scanner::FileEntry;

/// Reason a file was skipped during combining.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkipReason {
    /// Filename that was skipped.
    pub file: String,
    /// Human-readable explanation.
    pub reason: String,
}

/// Result of combining multiple data files.
pub struct CombineResult {
    /// Combined data with `_source` column appended.
    pub data: LoadedData,
    /// Number of files successfully combined.
    pub file_count: usize,
    /// Files that were skipped (with reasons).
    pub skipped: Vec<SkipReason>,
}

/// Combine data from multiple file entries.
///
/// Uses the first file's schema (column names + order) as the reference.
/// Appends `_source` column (filename stem) to each row.
/// Skips files with mismatched schemas or zero data rows.
#[allow(unused)]
pub fn combine_files(entries: &[FileEntry], no_header: bool) -> Result<CombineResult> {
    todo!()
}
