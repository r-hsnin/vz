//! File discovery and filtering for directory mode.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

/// Options controlling file discovery in a directory.
pub struct ScanOptions {
    /// Glob pattern to filter filenames (e.g. "sales_*.csv").
    pub glob_pattern: Option<String>,
    /// Recursively scan subdirectories (excludes hidden directories).
    pub recurse: bool,
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
/// Returns entries sorted lexicographically by filename (flat mode) or relative path (recursive mode).
/// Skips hidden files (starting with '.').
/// When `opts.recurse` is true, recursively traverses subdirectories (excluding hidden ones).
pub fn scan_directory(dir: &Path, opts: &ScanOptions) -> Result<Vec<FileEntry>> {
    let mut files: Vec<FileEntry> = Vec::new();
    collect_files(dir, dir, opts, &mut files)?;

    // Sort by stem (relative path for recursive, filename for flat) for determinism
    files.sort_by(|a, b| a.stem.cmp(&b.stem));

    if files.is_empty() {
        bail!("no data files found in {}", dir.display());
    }

    Ok(files)
}

/// Recursively collect data files from a directory tree.
fn collect_files(
    root: &Path,
    dir: &Path,
    opts: &ScanOptions,
    files: &mut Vec<FileEntry>,
) -> Result<()> {
    let entries = std::fs::read_dir(dir)
        .with_context(|| format!("cannot read directory: {}", dir.display()))?;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue, // skip unreadable entries
        };

        let path = entry.path();

        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => continue, // skip non-UTF8 filenames
        };

        // Skip hidden entries (files and directories)
        if filename.starts_with('.') {
            continue;
        }

        // Check if directory — use symlink_metadata to avoid following symlinked dirs
        let meta = match path.symlink_metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        if meta.is_dir() {
            if opts.recurse {
                collect_files(root, &path, opts, files)?;
            }
            continue;
        }

        // For files: follow symlinks to check if regular file
        let is_file = if meta.is_symlink() {
            path.metadata().map(|m| m.is_file()).unwrap_or(false)
        } else {
            meta.is_file()
        };
        if !is_file {
            continue;
        }

        // Filter by extension
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !DATA_EXTENSIONS.contains(&ext) {
            continue;
        }

        // Apply glob pattern if specified (matches against filename only)
        if let Some(ref pattern) = opts.glob_pattern {
            if !glob_matches(pattern, &filename) {
                continue;
            }
        }

        // Compute stem: relative path from root (without extension) for recursive,
        // or just filename stem for flat mode.
        let stem = if opts.recurse {
            let rel = path.strip_prefix(root).unwrap_or(&path);
            let rel_no_ext = rel.with_extension("");
            rel_no_ext.to_string_lossy().replace('\\', "/")
        } else {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string()
        };

        files.push(FileEntry { path, stem });
    }

    Ok(())
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
