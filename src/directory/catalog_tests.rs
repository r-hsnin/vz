//! Unit tests for catalog mode.

use std::path::Path;

use super::catalog::build_catalog;
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
fn test_catalog_same_schema_groups_all_files_together() {
    let entries = scan_directory(&fixture("same_schema"), &default_opts()).unwrap();
    let catalog = build_catalog(&entries, false);
    assert_eq!(catalog.groups.len(), 1);
    assert_eq!(catalog.groups[0].files.len(), 3);
    assert_eq!(catalog.groups[0].columns, vec!["date", "city", "revenue"]);
}

#[test]
fn test_catalog_mixed_schema_creates_multiple_groups() {
    let entries = scan_directory(&fixture("mixed_schema"), &default_opts()).unwrap();
    let catalog = build_catalog(&entries, false);
    assert_eq!(catalog.groups.len(), 2);
    // Largest group first (2 sales files)
    assert_eq!(catalog.groups[0].files.len(), 2);
    assert_eq!(catalog.groups[1].files.len(), 1);
}

#[test]
fn test_catalog_reports_correct_row_count_per_file() {
    let entries = scan_directory(&fixture("same_schema"), &default_opts()).unwrap();
    let catalog = build_catalog(&entries, false);
    for file in &catalog.groups[0].files {
        assert_eq!(file.row_count, 3, "each same_schema file has 3 rows");
    }
}

#[test]
fn test_catalog_reports_correct_format() {
    let entries = scan_directory(&fixture("mixed_extensions"), &default_opts()).unwrap();
    let catalog = build_catalog(&entries, false);
    let all_files: Vec<_> = catalog.groups.iter().flat_map(|g| &g.files).collect();
    let csv_file = all_files.iter().find(|f| f.stem == "data").unwrap();
    let tsv_file = all_files.iter().find(|f| f.stem == "report").unwrap();
    assert_eq!(csv_file.format, "csv");
    assert_eq!(tsv_file.format, "tsv");
}

#[test]
fn test_catalog_case_insensitive_grouping() {
    let entries = scan_directory(&fixture("case_insensitive"), &default_opts()).unwrap();
    let catalog = build_catalog(&entries, false);
    assert_eq!(
        catalog.groups.len(),
        1,
        "case differences should not split groups"
    );
}

#[test]
fn test_catalog_reordered_columns_same_group() {
    let entries = scan_directory(&fixture("reordered"), &default_opts()).unwrap();
    let catalog = build_catalog(&entries, false);
    assert_eq!(
        catalog.groups.len(),
        1,
        "reordered columns should group together"
    );
}

#[test]
fn test_catalog_header_only_file_shows_zero_rows() {
    let entries = scan_directory(&fixture("header_only"), &default_opts()).unwrap();
    let catalog = build_catalog(&entries, false);
    assert_eq!(catalog.groups.len(), 1);
    let file = &catalog.groups[0].files[0];
    assert_eq!(file.row_count, 0);
}

#[test]
fn test_catalog_single_file() {
    let entries = scan_directory(&fixture("single_file"), &default_opts()).unwrap();
    let catalog = build_catalog(&entries, false);
    assert_eq!(catalog.groups.len(), 1);
    assert_eq!(catalog.groups[0].files.len(), 1);
}

#[test]
fn test_catalog_recursive_includes_nested_files() {
    let opts = ScanOptions {
        glob_pattern: None,
        recurse: true,
    };
    let entries = scan_directory(&fixture("nested"), &opts).unwrap();
    let catalog = build_catalog(&entries, false);
    let total_files: usize = catalog.groups.iter().map(|g| g.files.len()).sum();
    assert!(total_files >= 4, "recursive should find nested files");
}

#[test]
fn test_catalog_recursive_relative_paths() {
    let opts = ScanOptions {
        glob_pattern: None,
        recurse: true,
    };
    let entries = scan_directory(&fixture("nested"), &opts).unwrap();
    let catalog = build_catalog(&entries, false);
    let all_stems: Vec<&str> = catalog
        .groups
        .iter()
        .flat_map(|g| g.files.iter().map(|f| f.stem.as_str()))
        .collect();
    // Should contain path-like stems for nested files
    assert!(
        all_stems.iter().any(|s| s.contains('/')),
        "recursive stems should include path separators: {:?}",
        all_stems
    );
}

#[test]
fn test_catalog_json_output_structure() {
    let entries = scan_directory(&fixture("mixed_schema"), &default_opts()).unwrap();
    let catalog = build_catalog(&entries, false);
    // Verify we can serialize to JSON without error
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
            serde_json::json!({
                "columns": g.columns,
                "files": files,
                "total_rows": g.files.iter().map(|f| f.row_count).sum::<usize>(),
            })
        })
        .collect();
    let output = serde_json::json!({
        "version": 1,
        "groups": groups_json,
        "errors": [],
    });
    assert_eq!(output["groups"].as_array().unwrap().len(), 2);
    assert_eq!(output["version"], 1);
}

#[test]
fn test_catalog_mixed_schema_correct_columns() {
    let entries = scan_directory(&fixture("mixed_schema"), &default_opts()).unwrap();
    let catalog = build_catalog(&entries, false);
    // Find the sales group (2 files)
    let sales_group = catalog.groups.iter().find(|g| g.files.len() == 2).unwrap();
    assert_eq!(sales_group.columns, vec!["date", "city", "revenue"]);
    // Find the users group (1 file)
    let users_group = catalog.groups.iter().find(|g| g.files.len() == 1).unwrap();
    assert_eq!(users_group.columns, vec!["name", "email", "age"]);
}
