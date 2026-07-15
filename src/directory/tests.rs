//! Unit tests for directory mode.

use std::path::Path;

use super::scanner::{ScanOptions, glob_matches, scan_directory};

fn default_opts() -> ScanOptions {
    ScanOptions { glob_pattern: None }
}

fn glob_opts(pattern: &str) -> ScanOptions {
    ScanOptions {
        glob_pattern: Some(pattern.to_string()),
    }
}

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/dir_test")
        .join(name)
}

// === Scanner tests ===

#[test]
fn test_scan_finds_all_data_files() {
    let result = scan_directory(&fixture("same_schema"), &default_opts()).unwrap();
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].stem, "sales_2024-01");
    assert_eq!(result[1].stem, "sales_2024-02");
    assert_eq!(result[2].stem, "sales_2024-03");
}

#[test]
fn test_scan_excludes_hidden_files() {
    let result = scan_directory(&fixture("with_hidden"), &default_opts()).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].stem, "visible");
}

#[test]
fn test_scan_excludes_non_data_extensions() {
    let result = scan_directory(&fixture("mixed_extensions"), &default_opts()).unwrap();
    assert_eq!(result.len(), 2);
    let stems: Vec<&str> = result.iter().map(|e| e.stem.as_str()).collect();
    assert!(stems.contains(&"data"));
    assert!(stems.contains(&"report"));
}

#[test]
fn test_scan_empty_directory_returns_error() {
    let result = scan_directory(&fixture("empty"), &default_opts());
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.to_lowercase().contains("no data files"),
        "unexpected error: {msg}"
    );
}

#[test]
fn test_scan_returns_sorted_lexicographically() {
    let result = scan_directory(&fixture("same_schema"), &default_opts()).unwrap();
    let stems: Vec<&str> = result.iter().map(|e| e.stem.as_str()).collect();
    let mut sorted = stems.clone();
    sorted.sort();
    assert_eq!(stems, sorted);
}

#[test]
fn test_scan_file_entry_paths_exist() {
    let result = scan_directory(&fixture("same_schema"), &default_opts()).unwrap();
    for entry in &result {
        assert!(entry.path.exists(), "path does not exist: {:?}", entry.path);
    }
}

// === Glob matcher tests ===

#[test]
fn test_glob_star_matches_all() {
    assert!(glob_matches("*", "anything.csv"));
    assert!(glob_matches("*", ""));
    assert!(glob_matches("*", "a"));
}

#[test]
fn test_glob_exact_match() {
    assert!(glob_matches("sales.csv", "sales.csv"));
    assert!(!glob_matches("sales.csv", "orders.csv"));
    assert!(!glob_matches("sales.csv", "sales.csv.bak"));
}

#[test]
fn test_glob_question_mark_single_char() {
    assert!(glob_matches("sales_?.csv", "sales_1.csv"));
    assert!(glob_matches("sales_?.csv", "sales_a.csv"));
    assert!(!glob_matches("sales_?.csv", "sales_12.csv"));
    assert!(!glob_matches("sales_?.csv", "sales_.csv"));
}

#[test]
fn test_glob_star_prefix() {
    assert!(glob_matches("*.csv", "data.csv"));
    assert!(glob_matches("*.csv", "long_name_file.csv"));
    assert!(!glob_matches("*.csv", "data.tsv"));
}

#[test]
fn test_glob_star_middle() {
    assert!(glob_matches("sales_*_2024.csv", "sales_jan_2024.csv"));
    assert!(glob_matches("sales_*_2024.csv", "sales_february_2024.csv"));
    assert!(!glob_matches("sales_*_2024.csv", "sales_jan_2023.csv"));
}

#[test]
fn test_glob_multiple_stars() {
    assert!(glob_matches("*sales*", "my_sales_data.csv"));
    assert!(glob_matches("*sales*", "sales"));
    assert!(!glob_matches("*sales*", "order_data.csv"));
}

#[test]
fn test_glob_empty_pattern_matches_empty() {
    assert!(glob_matches("", ""));
    assert!(!glob_matches("", "nonempty"));
}

#[test]
fn test_glob_no_wildcard_requires_exact() {
    assert!(glob_matches("report.tsv", "report.tsv"));
    assert!(!glob_matches("report.tsv", "report.csv"));
}

// === Glob integration with scan_directory ===

#[test]
fn test_scan_with_glob_filters_files() {
    let result = scan_directory(&fixture("same_schema"), &glob_opts("sales_2024-01*")).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].stem, "sales_2024-01");
}

#[test]
fn test_scan_with_glob_star_csv() {
    let result = scan_directory(&fixture("mixed_extensions"), &glob_opts("*.csv")).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].stem, "data");
}

#[test]
fn test_scan_with_glob_no_match_returns_error() {
    let result = scan_directory(&fixture("same_schema"), &glob_opts("nonexistent*"));
    assert!(result.is_err());
}
