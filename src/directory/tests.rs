//! Unit tests for directory mode.

use std::path::Path;

use super::scanner::{ScanOptions, scan_directory};

fn default_opts() -> ScanOptions {
    ScanOptions { glob_pattern: None }
}

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/dir_test")
        .join(name)
}

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
