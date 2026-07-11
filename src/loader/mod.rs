//! Unified data loader: CSV, TSV, JSON (array of objects), and NDJSON.
//!
//! Detects format from content (not just extension) and returns tabular data.

use anyhow::{Context, Result};
use std::io::Read;
use std::path::Path;

/// Raw loaded data before type inference.
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
        std::fs::read_to_string(path).with_context(|| format!("Failed to read file: {:?}", path))?
    };

    let format = format_override.unwrap_or_else(|| detect_format(path, &content));

    match format {
        InputFormat::Csv => load_delimited(&content, b',', no_header),
        InputFormat::Tsv => load_delimited(&content, b'\t', no_header),
        InputFormat::Json => load_json_array(&content),
        InputFormat::Ndjson => load_ndjson(&content),
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

    let mut rows: Vec<Vec<String>> = Vec::new();
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
mod tests {
    use super::*;
    use std::path::PathBuf;

    // --- Format Detection Tests ---

    #[test]
    fn test_detect_format_json_extension() {
        let path = PathBuf::from("data.json");
        assert_eq!(detect_format(&path, "anything"), InputFormat::Json);
    }

    #[test]
    fn test_detect_format_ndjson_extension() {
        let path = PathBuf::from("data.ndjson");
        assert_eq!(detect_format(&path, "anything"), InputFormat::Ndjson);
    }

    #[test]
    fn test_detect_format_jsonl_extension() {
        let path = PathBuf::from("data.jsonl");
        assert_eq!(detect_format(&path, "anything"), InputFormat::Ndjson);
    }

    #[test]
    fn test_detect_format_csv_extension() {
        let path = PathBuf::from("data.csv");
        assert_eq!(detect_format(&path, "anything"), InputFormat::Csv);
    }

    #[test]
    fn test_detect_format_tsv_extension() {
        let path = PathBuf::from("data.tsv");
        assert_eq!(detect_format(&path, "anything"), InputFormat::Tsv);
    }

    #[test]
    fn test_detect_format_json_array_content() {
        let path = PathBuf::from("-"); // stdin, no extension
        let content = r#"[{"x": 1, "y": 10}]"#;
        assert_eq!(detect_format(&path, content), InputFormat::Json);
    }

    #[test]
    fn test_detect_format_ndjson_content() {
        let path = PathBuf::from("-");
        let content = "{\"x\": 1}\n{\"x\": 2}\n";
        assert_eq!(detect_format(&path, content), InputFormat::Ndjson);
    }

    #[test]
    fn test_detect_format_csv_content() {
        let path = PathBuf::from("-");
        let content = "x,y\n1,10\n2,20";
        assert_eq!(detect_format(&path, content), InputFormat::Csv);
    }

    #[test]
    fn test_detect_format_tsv_content() {
        let path = PathBuf::from("-");
        let content = "x\ty\n1\t10\n2\t20";
        assert_eq!(detect_format(&path, content), InputFormat::Tsv);
    }

    #[test]
    fn test_detect_format_json_with_leading_whitespace() {
        let path = PathBuf::from("-");
        let content = "  \n  [{\"a\":1}]";
        assert_eq!(detect_format(&path, content), InputFormat::Json);
    }

    // --- JSON Loading Tests ---

    #[test]
    fn test_load_json_array_basic() {
        let content = r#"[
            {"name": "Alice", "score": 85},
            {"name": "Bob", "score": 92},
            {"name": "Charlie", "score": 78}
        ]"#;
        let data = load_json_array(content).unwrap();
        assert_eq!(data.headers.len(), 2);
        assert!(data.headers.contains(&"name".to_string()));
        assert!(data.headers.contains(&"score".to_string()));
        assert_eq!(data.rows.len(), 3);
    }

    #[test]
    fn test_load_json_array_numeric_values() {
        let content = r#"[{"x": 1, "y": 10.5}, {"x": 2, "y": 20.3}]"#;
        let data = load_json_array(content).unwrap();

        let x_idx = data.headers.iter().position(|h| h == "x").unwrap();
        let y_idx = data.headers.iter().position(|h| h == "y").unwrap();

        assert_eq!(data.rows[0][x_idx], "1");
        assert_eq!(data.rows[0][y_idx], "10.5");
        assert_eq!(data.rows[1][x_idx], "2");
        assert_eq!(data.rows[1][y_idx], "20.3");
    }

    #[test]
    fn test_load_json_array_with_nulls() {
        let content = r#"[{"a": 1, "b": null}, {"a": 2, "b": 3}]"#;
        let data = load_json_array(content).unwrap();

        let b_idx = data.headers.iter().position(|h| h == "b").unwrap();
        assert_eq!(data.rows[0][b_idx], ""); // null → empty string
        assert_eq!(data.rows[1][b_idx], "3");
    }

    #[test]
    fn test_load_json_array_empty() {
        let content = "[]";
        let data = load_json_array(content).unwrap();
        assert!(data.headers.is_empty());
        assert!(data.rows.is_empty());
    }

    #[test]
    fn test_load_json_array_string_dates() {
        let content = r#"[
            {"date": "2024-01-01", "value": 100},
            {"date": "2024-02-01", "value": 200}
        ]"#;
        let data = load_json_array(content).unwrap();
        let date_idx = data.headers.iter().position(|h| h == "date").unwrap();
        assert_eq!(data.rows[0][date_idx], "2024-01-01");
    }

    #[test]
    fn test_load_json_array_mixed_value_types() {
        let content = r#"[
            {"id": 1, "name": "Alice", "active": true, "score": 95.5},
            {"id": 2, "name": "Bob", "active": false, "score": 88.0}
        ]"#;
        let data = load_json_array(content).unwrap();
        let active_idx = data.headers.iter().position(|h| h == "active").unwrap();
        assert_eq!(data.rows[0][active_idx], "true");
        assert_eq!(data.rows[1][active_idx], "false");
    }

    // --- NDJSON Loading Tests ---

    #[test]
    fn test_load_ndjson_basic() {
        let content = "{\"x\": 1, \"y\": 10}\n{\"x\": 2, \"y\": 20}\n{\"x\": 3, \"y\": 30}\n";
        let data = load_ndjson(content).unwrap();
        assert_eq!(data.headers.len(), 2);
        assert_eq!(data.rows.len(), 3);
    }

    #[test]
    fn test_load_ndjson_with_empty_lines() {
        let content = "{\"a\": 1}\n\n{\"a\": 2}\n\n";
        let data = load_ndjson(content).unwrap();
        assert_eq!(data.rows.len(), 2);
    }

    #[test]
    fn test_load_ndjson_with_strings() {
        let content =
            "{\"city\": \"Tokyo\", \"pop\": 14000000}\n{\"city\": \"Osaka\", \"pop\": 2750000}\n";
        let data = load_ndjson(content).unwrap();
        let city_idx = data.headers.iter().position(|h| h == "city").unwrap();
        assert_eq!(data.rows[0][city_idx], "Tokyo");
        assert_eq!(data.rows[1][city_idx], "Osaka");
    }

    #[test]
    fn test_load_ndjson_missing_keys() {
        // Second object lacks "b" key
        let content = "{\"a\": 1, \"b\": 2}\n{\"a\": 3}\n";
        let data = load_ndjson(content).unwrap();
        let b_idx = data.headers.iter().position(|h| h == "b").unwrap();
        assert_eq!(data.rows[0][b_idx], "2");
        assert_eq!(data.rows[1][b_idx], ""); // missing → empty
    }

    // --- CSV/TSV Loading Tests ---

    #[test]
    fn test_load_delimited_csv() {
        let content = "name,score\nAlice,85\nBob,92\n";
        let data = load_delimited(content, b',', false).unwrap();
        assert_eq!(data.headers, vec!["name", "score"]);
        assert_eq!(data.rows.len(), 2);
        assert_eq!(data.rows[0], vec!["Alice", "85"]);
    }

    #[test]
    fn test_load_delimited_tsv() {
        let content = "city\tpop\nTokyo\t14000000\nOsaka\t2750000\n";
        let data = load_delimited(content, b'\t', false).unwrap();
        assert_eq!(data.headers, vec!["city", "pop"]);
        assert_eq!(data.rows.len(), 2);
    }

    #[test]
    fn test_load_delimited_no_header() {
        let content = "1,10\n2,20\n3,30\n";
        let data = load_delimited(content, b',', true).unwrap();
        assert_eq!(data.headers, vec!["col1", "col2"]);
        assert_eq!(data.rows.len(), 3);
        assert_eq!(data.rows[0], vec!["1", "10"]);
    }

    #[test]
    fn test_load_delimited_numeric_header_auto_detect() {
        // All-numeric "headers" are auto-detected as data
        let content = "1,100\n2,200\n3,300\n";
        let data = load_delimited(content, b',', false).unwrap();
        assert_eq!(data.headers, vec!["col1", "col2"]);
        assert_eq!(data.rows.len(), 3);
        assert_eq!(data.rows[0], vec!["1", "100"]);
    }

    #[test]
    fn test_headers_are_numeric() {
        assert!(headers_are_numeric(&["1".into(), "2".into(), "3.5".into()]));
        assert!(!headers_are_numeric(&["name".into(), "1".into()]));
        assert!(!headers_are_numeric(&[]));
    }

    // --- value_to_string Tests ---

    #[test]
    fn test_value_to_string_null() {
        assert_eq!(value_to_string(Some(&serde_json::Value::Null)), "");
    }

    #[test]
    fn test_value_to_string_none() {
        assert_eq!(value_to_string(None), "");
    }

    #[test]
    fn test_value_to_string_number() {
        let val = serde_json::json!(42);
        assert_eq!(value_to_string(Some(&val)), "42");
    }

    #[test]
    fn test_value_to_string_float() {
        let val = serde_json::json!(2.75);
        assert_eq!(value_to_string(Some(&val)), "2.75");
    }

    #[test]
    fn test_value_to_string_string() {
        let val = serde_json::json!("hello");
        assert_eq!(value_to_string(Some(&val)), "hello");
    }

    #[test]
    fn test_value_to_string_bool() {
        let val = serde_json::json!(true);
        assert_eq!(value_to_string(Some(&val)), "true");
    }

    #[test]
    fn test_expand_literal_escapes_single_line() {
        let input = "a,b\\n1,2\\n3,4\n".to_string();
        let result = expand_literal_escapes_if_needed(input);
        assert_eq!(result, "a,b\n1,2\n3,4\n");
    }

    #[test]
    fn test_expand_literal_escapes_no_trailing_newline() {
        let input = "a,b\\n1,2\\n3,4".to_string();
        let result = expand_literal_escapes_if_needed(input);
        assert_eq!(result, "a,b\n1,2\n3,4");
    }

    #[test]
    fn test_expand_literal_escapes_not_needed() {
        // Already has real newlines — don't expand
        let input = "a,b\n1,2\n3,4\n".to_string();
        let result = expand_literal_escapes_if_needed(input);
        assert_eq!(result, "a,b\n1,2\n3,4\n");
    }

    #[test]
    fn test_expand_literal_escapes_tab() {
        let input = "a\\tb\\n1\\t2\\n3\\t4\n".to_string();
        let result = expand_literal_escapes_if_needed(input);
        assert_eq!(result, "a\tb\n1\t2\n3\t4\n");
    }
}

#[cfg(test)]
mod sampling_tests {
    use super::*;

    #[test]
    fn test_apply_sampling_no_op_when_under_limit() {
        let data = LoadedData {
            headers: vec!["x".into()],
            rows: vec![vec!["1".into()], vec!["2".into()], vec!["3".into()]],
        };
        let result = apply_sampling(data, 10);
        assert_eq!(result.rows.len(), 3);
    }

    #[test]
    fn test_apply_sampling_reduces_rows() {
        let data = LoadedData {
            headers: vec!["x".into()],
            rows: (0..100).map(|i| vec![format!("{}", i)]).collect(),
        };
        let result = apply_sampling(data, 10);
        assert_eq!(result.rows.len(), 10);
        // First row should be row 0, last should be close to row 90
        assert_eq!(result.rows[0][0], "0");
        assert_eq!(result.rows[9][0], "90");
    }

    #[test]
    fn test_apply_sampling_preserves_headers() {
        let data = LoadedData {
            headers: vec!["name".into(), "value".into()],
            rows: (0..50)
                .map(|i| vec![format!("n{}", i), format!("{}", i)])
                .collect(),
        };
        let result = apply_sampling(data, 5);
        assert_eq!(result.headers, vec!["name", "value"]);
        assert_eq!(result.rows.len(), 5);
    }
}
