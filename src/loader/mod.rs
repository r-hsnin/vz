//! Unified data loader: CSV, TSV, JSON (array of objects), NDJSON, and fixed-width.
//!
//! Detects format from content (not just extension) and returns tabular data.

use anyhow::{Context, Result};
use std::io::Read;
use std::path::Path;

pub mod space;

/// Raw loaded data before type inference.
#[derive(Debug)]
pub struct LoadedData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// Detected input format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputFormat {
    Csv,
    Tsv,
    Json,
    Ndjson,
    Space,
}

/// Detect the input format from file extension and content.
pub fn detect_format(path: &Path, content: &str) -> InputFormat {
    // Check extension first
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        match ext.to_lowercase().as_str() {
            "json" => return InputFormat::Json,
            "ndjson" | "jsonl" => return InputFormat::Ndjson,
            "tsv" | "tab" => return InputFormat::Tsv,
            "csv" => return InputFormat::Csv,
            _ => {}
        }
    }

    // Content-based detection
    let trimmed = content.trim_start();
    if trimmed.starts_with('[') {
        return InputFormat::Json;
    }
    if trimmed.starts_with('{') {
        return InputFormat::Ndjson;
    }

    // TSV vs CSV heuristic
    if let Some(first_line) = content.lines().next() {
        let tabs = first_line.chars().filter(|c| *c == '\t').count();
        let commas = first_line.chars().filter(|c| *c == ',').count();
        if tabs > 0 && tabs >= commas {
            return InputFormat::Tsv;
        }
    }

    // Fixed-width / space-aligned format detection
    if space::looks_like_space_format(content) {
        return InputFormat::Space;
    }

    InputFormat::Csv
}

/// Load data from a file path (or stdin if "-").
/// Auto-detects format: CSV, TSV, JSON, or NDJSON.
pub fn load_data(path: &Path) -> Result<LoadedData> {
    load_data_opts(path, false)
}

/// Load data with header options.
/// If `no_header` is true, treat first row as data (generate synthetic headers).
/// If false but first row is all-numeric, auto-detect as headerless.
pub fn load_data_opts(path: &Path, no_header: bool) -> Result<LoadedData> {
    load_data_full(path, no_header, None)
}

/// Load data directly from a content string with explicit format.
/// Useful for benchmarks and tests that want to avoid file I/O.
pub fn load_from_content(
    content: &str,
    format: InputFormat,
    no_header: bool,
) -> Result<LoadedData> {
    match format {
        InputFormat::Csv => load_delimited(content, b',', no_header),
        InputFormat::Tsv => load_delimited(content, b'\t', no_header),
        InputFormat::Json => load_json_array(content),
        InputFormat::Ndjson => load_ndjson(content),
        InputFormat::Space => space::load_space(content, no_header),
    }
}

/// Load data with full options: header control and format override.
pub fn load_data_full(
    path: &Path,
    no_header: bool,
    format_override: Option<InputFormat>,
) -> Result<LoadedData> {
    let content = if path == Path::new("-") {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        expand_literal_escapes_if_needed(buf)
    } else {
        std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?
    };

    let format = format_override.unwrap_or_else(|| detect_format(path, &content));

    match format {
        InputFormat::Csv => load_delimited(&content, b',', no_header),
        InputFormat::Tsv => load_delimited(&content, b'\t', no_header),
        InputFormat::Json => load_json_array(&content),
        InputFormat::Ndjson => load_ndjson(&content),
        InputFormat::Space => space::load_space(&content, no_header),
    }
}

/// Load CSV/TSV data with given delimiter.
/// If `no_header` is true or first row is all-numeric, treat first row as data.
fn load_delimited(content: &str, delimiter: u8, no_header: bool) -> Result<LoadedData> {
    if no_header {
        return load_delimited_no_header(content, delimiter);
    }

    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .flexible(true)
        .from_reader(content.as_bytes());

    let headers: Vec<String> = rdr
        .headers()
        .context("Failed to read CSV headers")?
        .iter()
        .map(|s| s.to_string())
        .collect();

    // Auto-detect: if all "headers" look numeric, treat first row as data
    if headers_are_numeric(&headers) {
        return load_delimited_no_header(content, delimiter);
    }

    // Estimate row count from content length for capacity hint
    let estimated_rows = content.len() / 40; // ~40 bytes per row heuristic
    let mut rows: Vec<Vec<String>> = Vec::with_capacity(estimated_rows);
    for (i, result) in rdr.records().enumerate() {
        match result {
            Ok(record) => rows.push(record.iter().map(|s| s.to_string()).collect()),
            Err(e) => eprintln!("warning: skipping row {}: {}", i + 2, e),
        }
    }

    Ok(LoadedData { headers, rows })
}

/// Check if all header values parse as numbers (indicating they're probably data, not headers).
fn headers_are_numeric(headers: &[String]) -> bool {
    !headers.is_empty() && headers.iter().all(|h| h.parse::<f64>().is_ok())
}

/// Load delimited data treating ALL rows as data (no header row).
/// Generates synthetic column names: col1, col2, ...
fn load_delimited_no_header(content: &str, delimiter: u8) -> Result<LoadedData> {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(false)
        .flexible(true)
        .from_reader(content.as_bytes());

    let mut rows: Vec<Vec<String>> = Vec::new();
    for (i, result) in rdr.records().enumerate() {
        match result {
            Ok(record) => rows.push(record.iter().map(|s| s.to_string()).collect()),
            Err(e) => eprintln!("warning: skipping row {}: {}", i + 1, e),
        }
    }

    let col_count = rows.first().map(|r| r.len()).unwrap_or(0);
    let headers: Vec<String> = (1..=col_count).map(|i| format!("col{}", i)).collect();

    Ok(LoadedData { headers, rows })
}

/// Load a JSON array of objects.
/// Expected format: [{"col1": val1, "col2": val2}, ...]
fn load_json_array(content: &str) -> Result<LoadedData> {
    let trimmed = content.trim_start();
    if trimmed.starts_with('{') {
        anyhow::bail!(
            "Input is a single JSON object, not an array.\n\n  \
             vz expects a JSON array of objects: [{{\"col\": \"val\"}}, ...]\n  \
             Tip: wrap in brackets, or use NDJSON format (-f ndjson) for single objects."
        );
    }
    let values: Vec<serde_json::Value> =
        serde_json::from_str(content).context("Failed to parse JSON array")?;
    objects_to_tabular(values)
}

/// Load NDJSON (newline-delimited JSON) — one JSON object per line.
fn load_ndjson(content: &str) -> Result<LoadedData> {
    let objects: Vec<serde_json::Value> = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("Failed to parse NDJSON")?;
    objects_to_tabular(objects)
}

/// Convert a list of JSON objects into tabular (headers + rows) format.
fn objects_to_tabular(objects: Vec<serde_json::Value>) -> Result<LoadedData> {
    if objects.is_empty() {
        return Ok(LoadedData {
            headers: vec![],
            rows: vec![],
        });
    }

    let headers: Vec<String> = match &objects[0] {
        serde_json::Value::Object(map) => map.keys().cloned().collect(),
        _ => anyhow::bail!(
            "JSON elements must be objects (e.g., [{{\"col\": \"val\"}}]). Got an array of primitives."
        ),
    };

    let rows: Vec<Vec<String>> = objects
        .iter()
        .filter_map(|v| {
            let obj = v.as_object()?;
            Some(
                headers
                    .iter()
                    .map(|h| value_to_string(obj.get(h)))
                    .collect(),
            )
        })
        .collect();

    Ok(LoadedData { headers, rows })
}

/// Convert a JSON value to a string suitable for type inference.
fn value_to_string(val: Option<&serde_json::Value>) -> String {
    match val {
        None | Some(serde_json::Value::Null) => String::new(),
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Number(n)) => n.to_string(),
        Some(serde_json::Value::Bool(b)) => b.to_string(),
        Some(other) => other.to_string(),
    }
}

/// If stdin content appears to contain literal `\n` or `\t` escape sequences
/// (not expanded by the shell), auto-expand them to real newlines/tabs.
/// Heuristic: the content has very few real newlines relative to literal `\n` occurrences.
fn expand_literal_escapes_if_needed(content: String) -> String {
    let real_newlines = content.chars().filter(|&c| c == '\n').count();
    let literal_backslash_n = content.matches("\\n").count();

    // Only expand if there are literal \n sequences and few real newlines.
    // "Few" means at most 1 real newline (the trailing one from echo).
    if literal_backslash_n >= 1 && real_newlines <= 1 {
        content.replace("\\n", "\n").replace("\\t", "\t")
    } else {
        content
    }
}

/// Apply systematic sampling to loaded data, reducing to at most `max_rows`.
/// Uses systematic (every-Nth) sampling to preserve distribution across the dataset.
/// Prints a warning to stderr indicating how many rows were sampled.
pub fn apply_sampling(mut data: LoadedData, max_rows: usize) -> LoadedData {
    let total = data.rows.len();
    if total <= max_rows {
        return data;
    }

    let step = total as f64 / max_rows as f64;
    let sampled: Vec<Vec<String>> = (0..max_rows)
        .map(|i| {
            let idx = (i as f64 * step).floor() as usize;
            data.rows[idx.min(total - 1)].clone()
        })
        .collect();

    eprintln!(
        "info: sampled {}/{} rows (systematic)",
        sampled.len(),
        total
    );
    data.rows = sampled;
    data
}

#[cfg(test)]
mod tests;
