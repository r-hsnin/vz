//! Catalog mode: display schema inventory of files in a directory without combining.

use anyhow::Result;

use crate::cli::Cli;
use crate::loader;

use super::scanner::FileEntry;

/// Information about a single file in the catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileInfo {
    /// Filename stem (or relative path for recursive mode).
    pub stem: String,
    /// Detected format name (csv, tsv, json, ndjson).
    pub format: String,
    /// Column names from this file.
    pub columns: Vec<String>,
    /// Number of data rows (excluding header).
    pub row_count: usize,
}

/// A group of files sharing the same schema (same columns, case-insensitive).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaGroup {
    /// Canonical column names (from the first file in the group).
    pub columns: Vec<String>,
    /// Normalized sorted key for grouping.
    pub key: String,
    /// Files belonging to this group.
    pub files: Vec<FileInfo>,
}

/// Error entry for files that failed to load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogError {
    /// Filename stem.
    pub stem: String,
    /// Error description.
    pub error: String,
}

/// Full catalog result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogResult {
    pub groups: Vec<SchemaGroup>,
    pub errors: Vec<CatalogError>,
}

/// Normalize a column name for grouping (lowercase, trimmed).
fn normalize_col(col: &str) -> String {
    col.trim().to_lowercase()
}

/// Compute the schema key for grouping: sorted normalized column names joined.
fn schema_key(columns: &[String]) -> String {
    let mut normalized: Vec<String> = columns.iter().map(|c| normalize_col(c)).collect();
    normalized.sort();
    normalized.join(",")
}

/// Build a catalog from scanned file entries.
///
/// Loads each file's headers and row count, then groups by matching schema.
pub fn build_catalog(entries: &[FileEntry], no_header: bool) -> CatalogResult {
    let mut infos: Vec<FileInfo> = Vec::new();
    let mut errors: Vec<CatalogError> = Vec::new();

    for entry in entries {
        match loader::load_data_full(&entry.path, no_header, None) {
            Ok(data) => {
                let format = loader::detect_format(
                    &entry.path,
                    &std::fs::read_to_string(&entry.path).unwrap_or_default(),
                );
                let format_name = match format {
                    loader::InputFormat::Csv => "csv",
                    loader::InputFormat::Tsv => "tsv",
                    loader::InputFormat::Json => "json",
                    loader::InputFormat::Ndjson => "ndjson",
                    loader::InputFormat::Space => "space",
                };
                infos.push(FileInfo {
                    stem: entry.stem.clone(),
                    format: format_name.to_string(),
                    columns: data.headers,
                    row_count: data.rows.len(),
                });
            }
            Err(e) => {
                errors.push(CatalogError {
                    stem: entry.stem.clone(),
                    error: format!("{e}"),
                });
            }
        }
    }

    // Group by schema key
    let mut groups: Vec<SchemaGroup> = Vec::new();
    for info in infos {
        let key = schema_key(&info.columns);
        if let Some(group) = groups.iter_mut().find(|g| g.key == key) {
            group.files.push(info);
        } else {
            let columns = info.columns.clone();
            groups.push(SchemaGroup {
                columns,
                key,
                files: vec![info],
            });
        }
    }

    // Sort groups by file count descending, then by key for stability
    groups.sort_by(|a, b| b.files.len().cmp(&a.files.len()).then(a.key.cmp(&b.key)));

    CatalogResult { groups, errors }
}

/// Print catalog in human-readable text format to stdout.
pub fn print_catalog_text(catalog: &CatalogResult) {
    for (i, group) in catalog.groups.iter().enumerate() {
        let total_rows: usize = group.files.iter().map(|f| f.row_count).sum();
        let file_word = if group.files.len() == 1 {
            "file"
        } else {
            "files"
        };
        println!(
            "Schema {} ({} {}): {}",
            (b'A' + i as u8) as char,
            group.files.len(),
            file_word,
            group.columns.join(", ")
        );
        for f in &group.files {
            println!("  {:<30} {:<8} {} rows", f.stem, f.format, f.row_count);
        }
        println!("  Total: {} rows", total_rows);
        println!();
    }

    if !catalog.errors.is_empty() {
        println!("Errors:");
        for err in &catalog.errors {
            println!("  {}: {}", err.stem, err.error);
        }
        println!();
    }
}

/// Print catalog as JSON to stdout.
pub fn print_catalog_json(catalog: &CatalogResult) -> Result<()> {
    let groups_json: Vec<serde_json::Value> = catalog
        .groups
        .iter()
        .map(|g| {
            let files: Vec<serde_json::Value> = g
                .files
                .iter()
                .map(|f| {
                    serde_json::json!({
                        "name": f.stem,
                        "format": f.format,
                        "rows": f.row_count,
                    })
                })
                .collect();
            let total_rows: usize = g.files.iter().map(|f| f.row_count).sum();
            serde_json::json!({
                "columns": g.columns,
                "files": files,
                "total_rows": total_rows,
            })
        })
        .collect();

    let errors_json: Vec<serde_json::Value> = catalog
        .errors
        .iter()
        .map(|e| {
            serde_json::json!({
                "name": e.stem,
                "error": e.error,
            })
        })
        .collect();

    let output = serde_json::json!({
        "version": 1,
        "groups": groups_json,
        "errors": errors_json,
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Run catalog mode: scan directory, build catalog, output.
pub fn run_catalog(cli: &Cli, entries: &[FileEntry]) -> Result<()> {
    let catalog = build_catalog(entries, cli.no_header);

    if catalog.groups.is_empty() {
        anyhow::bail!("no readable data files found for catalog");
    }

    if cli.output == Some(crate::cli::OutputFormat::Json) {
        print_catalog_json(&catalog)?;
    } else {
        print_catalog_text(&catalog);
    }

    Ok(())
}
