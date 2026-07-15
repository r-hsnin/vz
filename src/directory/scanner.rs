//! File discovery and filtering for directory mode.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

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
// Keep in sync with loader's detect_format() supported formats.
const DATA_EXTENSIONS: &[&str] = &["csv", "tsv", "json", "ndjson", "jsonl", "tab"];

/// Scan a directory for data files.
///
/// Returns entries sorted lexicographically by filename.
/// Skips hidden files (starting with '.').
/// Only scans one level deep (no recursion).
pub fn scan_directory(dir: &Path, opts: &ScanOptions) -> Result<Vec<FileEntry>> {
    let entries = std::fs::read_dir(dir)
        .with_context(|| format!("cannot read directory: {}", dir.display()))?;

    let mut files: Vec<FileEntry> = Vec::new();

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue, // skip unreadable entries
        };

        let path = entry.path();

        // Only process regular files (follows symlinks)
        let is_file = path.metadata().map(|m| m.is_file()).unwrap_or(false);
        if !is_file {
            continue;
        }

        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => continue, // skip non-UTF8 filenames
        };

        // Exclude hidden files
        if filename.starts_with('.') {
            continue;
        }

        // Filter by extension
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !DATA_EXTENSIONS.contains(&ext) {
            continue;
        }

        // Apply glob pattern if specified
        if let Some(ref pattern) = opts.glob_pattern {
            if !glob_matches(pattern, &filename) {
                continue;
            }
        }

        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        files.push(FileEntry { path, stem });
    }

    // Sort lexicographically by filename
    files.sort_by(|a, b| {
        let name_a = a.path.file_name().unwrap_or_default();
        let name_b = b.path.file_name().unwrap_or_default();
        name_a.cmp(name_b)
    });

    if files.is_empty() {
        bail!("no data files found in {}", dir.display());
    }

    Ok(files)
}

/// Match a filename against a glob pattern supporting `*` and `?`.
///
/// `*` matches zero or more characters.
/// `?` matches exactly one character.
pub(super) fn glob_matches(pattern: &str, filename: &str) -> bool {
    let p = pattern.as_bytes();
    let s = filename.as_bytes();
    let (mut pi, mut si) = (0usize, 0usize);
    let (mut star_pi, mut star_si) = (usize::MAX, 0usize);

    while si < s.len() {
        if pi < p.len() && (p[pi] == b'?' || p[pi] == s[si]) {
            pi += 1;
            si += 1;
        } else if pi < p.len() && p[pi] == b'*' {
            star_pi = pi;
            star_si = si;
            pi += 1;
        } else if star_pi != usize::MAX {
            pi = star_pi + 1;
            star_si += 1;
            si = star_si;
        } else {
            return false;
        }
    }

    // Consume trailing stars
    while pi < p.len() && p[pi] == b'*' {
        pi += 1;
    }

    pi == p.len()
}
