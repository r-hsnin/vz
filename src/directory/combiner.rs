//! Schema matching and file combining for directory mode.

use anyhow::{Result, bail};

use crate::loader::{self, LoadedData};

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
#[derive(Debug)]
pub struct CombineResult {
    /// Combined data with `_source` column appended.
    pub data: LoadedData,
    /// Number of files successfully combined.
    pub file_count: usize,
    /// Files that were skipped (with reasons).
    pub skipped: Vec<SkipReason>,
}

/// Normalize a header name for case-insensitive comparison.
///
/// Trims whitespace and converts to lowercase. Does not modify the original.
fn normalize_header(h: &str) -> String {
    h.trim().to_lowercase()
}

/// Build a column reorder map from incoming headers to reference headers.
///
/// Returns `Some(indices)` where `indices[ref_pos]` is the position in `incoming`
/// that maps to `reference[ref_pos]`. Returns `None` if headers don't match
/// (case-insensitive, after trimming).
fn build_reorder_map(reference: &[String], incoming: &[String]) -> Option<Vec<usize>> {
    if reference.len() != incoming.len() {
        return None;
    }

    let normalized_incoming: Vec<String> = incoming.iter().map(|h| normalize_header(h)).collect();

    let mut indices = Vec::with_capacity(reference.len());
    for ref_h in reference {
        let target = normalize_header(ref_h);
        let pos = normalized_incoming.iter().position(|h| *h == target)?;
        indices.push(pos);
    }

    Some(indices)
}

/// Combine data from multiple file entries.
///
/// Uses the first file's schema (column names + order) as the reference.
/// Headers are compared case-insensitively with whitespace trimming.
/// Appends `_source` column (filename stem) to each row.
/// Skips files with mismatched schemas or zero data rows.
pub fn combine_files(entries: &[FileEntry], no_header: bool) -> Result<CombineResult> {
    if entries.is_empty() {
        bail!("no file entries to combine");
    }

    let mut reference_headers: Option<Vec<String>> = None;
    let mut combined_rows: Vec<Vec<String>> = Vec::new();
    let mut file_count = 0usize;
    let mut skipped: Vec<SkipReason> = Vec::new();

    for entry in entries {
        let data = match loader::load_data_full(&entry.path, no_header, None) {
            Ok(d) => d,
            Err(e) => {
                if reference_headers.is_none() {
                    // First file must load successfully
                    bail!(
                        "failed to load first file '{}': {}",
                        entry.path.display(),
                        e
                    );
                }
                skipped.push(SkipReason {
                    file: entry.stem.clone(),
                    reason: format!("load error: {e}"),
                });
                continue;
            }
        };

        // Skip files with zero data rows
        if data.rows.is_empty() {
            skipped.push(SkipReason {
                file: entry.stem.clone(),
                reason: "header only (0 data rows)".to_string(),
            });
            continue;
        }

        // Schema comparison (case-insensitive, whitespace-trimmed, order-independent)
        match &reference_headers {
            None => {
                // First valid file sets the reference schema
                reference_headers = Some(data.headers.clone());
            }
            Some(ref_headers) => {
                let reorder_map = build_reorder_map(ref_headers, &data.headers);
                match reorder_map {
                    None => {
                        skipped.push(SkipReason {
                            file: entry.stem.clone(),
                            reason: format!(
                                "schema mismatch (expected [{}], got [{}])",
                                ref_headers.join(", "),
                                data.headers.join(", ")
                            ),
                        });
                        continue;
                    }
                    Some(ref map) => {
                        // Check if columns need reordering
                        let needs_reorder = map.iter().enumerate().any(|(i, &m)| i != m);
                        if needs_reorder {
                            // Append rows with columns remapped to reference order
                            for row in &data.rows {
                                let mut reordered: Vec<String> =
                                    map.iter().map(|&i| row[i].clone()).collect();
                                reordered.push(entry.stem.clone());
                                combined_rows.push(reordered);
                            }
                        } else {
                            // Columns already in correct order
                            for mut row in data.rows {
                                row.push(entry.stem.clone());
                                combined_rows.push(row);
                            }
                        }
                        file_count += 1;
                        continue;
                    }
                }
            }
        }

        // First file: append rows with _source column (no reordering needed)
        for mut row in data.rows {
            row.push(entry.stem.clone());
            combined_rows.push(row);
        }
        file_count += 1;
    }

    let headers = match reference_headers {
        Some(mut h) => {
            h.push("_source".to_string());
            h
        }
        None => bail!("no files with data rows found"),
    };

    if file_count == 0 {
        bail!("no files with matching schema could be combined");
    }

    Ok(CombineResult {
        data: LoadedData {
            headers,
            rows: combined_rows,
        },
        file_count,
        skipped,
    })
}
