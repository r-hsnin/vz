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

#[test]
fn test_load_ndjson_nested_objects_serialized() {
    // Nested objects/arrays should be serialized as JSON strings (not panic)
    let content = r#"{"name": "Alice", "address": {"city": "Tokyo", "zip": "100"}}
{"name": "Bob", "address": {"city": "Osaka", "zip": "530"}}
"#;
    let data = load_ndjson(content).unwrap();
    let addr_idx = data.headers.iter().position(|h| h == "address").unwrap();
    let name_idx = data.headers.iter().position(|h| h == "name").unwrap();
    assert_eq!(data.rows[0][name_idx], "Alice");
    // Nested object rendered as JSON string
    let addr_val = &data.rows[0][addr_idx];
    assert!(
        addr_val.contains("Tokyo"),
        "Nested object should contain city value, got: {}",
        addr_val
    );
    assert!(
        addr_val.contains("zip"),
        "Nested object should preserve structure, got: {}",
        addr_val
    );
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

// --- Sampling Tests ---

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

#[test]
fn test_bom_csv_headers_clean() {
    let content = "\u{feff}date,city,revenue\n2024-01-01,Tokyo,1000\n2024-02-01,Osaka,1500\n";
    let data = load_from_content(content, InputFormat::Csv, false).unwrap();
    assert_eq!(data.headers[0], "date");
    assert_eq!(data.headers[1], "city");
    assert_eq!(data.headers[2], "revenue");
    assert_eq!(data.rows.len(), 2);
}

#[test]
fn test_bom_tsv_headers_clean() {
    let content = "\u{feff}name\tscore\nAlice\t85\nBob\t92\n";
    let data = load_from_content(content, InputFormat::Tsv, false).unwrap();
    assert_eq!(data.headers[0], "name");
    assert_eq!(data.headers[1], "score");
    assert_eq!(data.rows.len(), 2);
}

#[test]
fn test_bom_json_parsed() {
    let content = "\u{feff}[{\"x\": 1, \"y\": 2}]";
    let data = load_from_content(content, InputFormat::Json, false).unwrap();
    assert!(data.headers.contains(&"x".to_string()));
    assert!(data.headers.contains(&"y".to_string()));
    assert_eq!(data.rows.len(), 1);
}

#[test]
fn test_no_bom_csv_unchanged() {
    let content = "date,city,revenue\n2024-01-01,Tokyo,1000\n";
    let data = load_from_content(content, InputFormat::Csv, false).unwrap();
    assert_eq!(data.headers[0], "date");
}

#[test]
fn test_bom_no_header_mode() {
    let content = "\u{feff}1,10\n2,20\n3,30\n";
    let data = load_from_content(content, InputFormat::Csv, false).unwrap();
    // All-numeric headers → auto-detect as headerless → synthetic headers
    assert_eq!(data.headers[0], "col1");
    assert_eq!(data.rows.len(), 3);
}
