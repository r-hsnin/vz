//! Unit tests for mixed format combining in directory mode (Cycle 5).

use std::path::Path;

use super::combiner::combine_files;
use super::scanner::{ScanOptions, scan_directory};

fn default_opts() -> ScanOptions {
    ScanOptions {
        glob_pattern: None,
        recurse: false,
    }
}

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/dir_test")
        .join(name)
}

#[test]
fn test_combine_mixed_csv_json_tsv_same_schema() {
    let entries = scan_directory(&fixture("mixed_format"), &default_opts()).unwrap();
    let result = combine_files(&entries, false).unwrap();

    // 3 files (sales.csv, stats.json, summary.tsv) all combine
    assert_eq!(result.file_count, 3);
    assert!(result.skipped.is_empty());
}

#[test]
fn test_combine_mixed_format_total_row_count() {
    let entries = scan_directory(&fixture("mixed_format"), &default_opts()).unwrap();
    let result = combine_files(&entries, false).unwrap();

    // Each file has 2 rows → 6 total
    assert_eq!(result.data.rows.len(), 6);
}

#[test]
fn test_combine_mixed_format_headers_from_csv_first() {
    // "sales.csv" sorts lexicographically before "stats.json" and "summary.tsv"
    // So CSV sets the reference schema: date, city, revenue
    let entries = scan_directory(&fixture("mixed_format"), &default_opts()).unwrap();
    let result = combine_files(&entries, false).unwrap();

    assert_eq!(result.data.headers[0], "date");
    assert_eq!(result.data.headers[1], "city");
    assert_eq!(result.data.headers[2], "revenue");
    assert_eq!(result.data.headers[3], "_source");
    assert_eq!(result.data.headers[4], "_file_date");
}

#[test]
fn test_combine_mixed_format_json_columns_reordered() {
    // JSON keys via BTreeMap are alphabetical: city, date, revenue
    // But reference is CSV order: date, city, revenue
    // Data from JSON file should be correctly remapped
    let entries = scan_directory(&fixture("mixed_format"), &default_opts()).unwrap();
    let result = combine_files(&entries, false).unwrap();

    // stats.json rows (file index 1, rows index 2-3)
    let json_rows: Vec<&Vec<String>> = result
        .data
        .rows
        .iter()
        .filter(|r| r[3] == "stats")
        .collect();
    assert_eq!(json_rows.len(), 2);
    // First JSON row: date=2024-02-01, city=Nagoya, revenue=300
    assert_eq!(json_rows[0][0], "2024-02-01");
    assert_eq!(json_rows[0][1], "Nagoya");
    assert_eq!(json_rows[0][2], "300");
}

#[test]
fn test_combine_mixed_format_source_column_correct() {
    let entries = scan_directory(&fixture("mixed_format"), &default_opts()).unwrap();
    let result = combine_files(&entries, false).unwrap();

    let sources: Vec<&str> = result.data.rows.iter().map(|r| r[3].as_str()).collect();
    // 2 rows from each file, in lex order: sales, stats, summary
    assert_eq!(
        sources,
        vec!["sales", "sales", "stats", "stats", "summary", "summary"]
    );
}

#[test]
fn test_combine_mixed_format_csv_data_integrity() {
    let entries = scan_directory(&fixture("mixed_format"), &default_opts()).unwrap();
    let result = combine_files(&entries, false).unwrap();

    // First 2 rows are from sales.csv
    assert_eq!(result.data.rows[0][0], "2024-01-01");
    assert_eq!(result.data.rows[0][1], "Tokyo");
    assert_eq!(result.data.rows[0][2], "100");
    assert_eq!(result.data.rows[1][0], "2024-01-15");
    assert_eq!(result.data.rows[1][1], "Osaka");
    assert_eq!(result.data.rows[1][2], "200");
}

#[test]
fn test_combine_mixed_format_tsv_data_integrity() {
    let entries = scan_directory(&fixture("mixed_format"), &default_opts()).unwrap();
    let result = combine_files(&entries, false).unwrap();

    // Last 2 rows are from summary.tsv
    let tsv_rows: Vec<&Vec<String>> = result
        .data
        .rows
        .iter()
        .filter(|r| r[3] == "summary")
        .collect();
    assert_eq!(tsv_rows[0][0], "2024-03-01");
    assert_eq!(tsv_rows[0][1], "Osaka");
    assert_eq!(tsv_rows[0][2], "500");
    assert_eq!(tsv_rows[1][0], "2024-03-15");
    assert_eq!(tsv_rows[1][1], "Nagoya");
    assert_eq!(tsv_rows[1][2], "600");
}

#[test]
fn test_combine_mixed_format_all_rows_have_correct_column_count() {
    let entries = scan_directory(&fixture("mixed_format"), &default_opts()).unwrap();
    let result = combine_files(&entries, false).unwrap();

    for (i, row) in result.data.rows.iter().enumerate() {
        assert_eq!(
            row.len(),
            result.data.headers.len(),
            "row {i} has wrong column count"
        );
    }
}
