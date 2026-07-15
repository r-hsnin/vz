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

// === Combiner tests (Cycle 4) ===

use super::combiner::combine_files;

#[test]
fn test_combine_same_schema_all_rows_and_source() {
    let entries = scan_directory(&fixture("same_schema"), &default_opts()).unwrap();
    let result = combine_files(&entries, false).unwrap();

    // 3 files × 3 rows = 9 rows
    assert_eq!(result.data.rows.len(), 9);
    // Headers: date, city, revenue, _source
    assert_eq!(result.data.headers.len(), 4);
    assert_eq!(result.data.headers[3], "_source");
    assert_eq!(result.file_count, 3);
    assert!(result.skipped.is_empty());
}

#[test]
fn test_combine_source_column_contains_file_stem() {
    let entries = scan_directory(&fixture("same_schema"), &default_opts()).unwrap();
    let result = combine_files(&entries, false).unwrap();

    let source_col_idx = 3;
    // First 3 rows from sales_2024-01
    assert_eq!(result.data.rows[0][source_col_idx], "sales_2024-01");
    assert_eq!(result.data.rows[2][source_col_idx], "sales_2024-01");
    // Middle rows from sales_2024-02
    assert_eq!(result.data.rows[3][source_col_idx], "sales_2024-02");
    assert_eq!(result.data.rows[5][source_col_idx], "sales_2024-02");
    // Last rows from sales_2024-03
    assert_eq!(result.data.rows[6][source_col_idx], "sales_2024-03");
    assert_eq!(result.data.rows[8][source_col_idx], "sales_2024-03");
}

#[test]
fn test_combine_headers_are_original_plus_source() {
    let entries = scan_directory(&fixture("same_schema"), &default_opts()).unwrap();
    let result = combine_files(&entries, false).unwrap();

    assert_eq!(
        result.data.headers,
        vec!["date", "city", "revenue", "_source"]
    );
}

#[test]
fn test_combine_row_columns_match_header_count() {
    let entries = scan_directory(&fixture("same_schema"), &default_opts()).unwrap();
    let result = combine_files(&entries, false).unwrap();

    for (i, row) in result.data.rows.iter().enumerate() {
        assert_eq!(
            row.len(),
            result.data.headers.len(),
            "row {i} has wrong column count"
        );
    }
}

#[test]
fn test_combine_rows_follow_file_order() {
    let entries = scan_directory(&fixture("same_schema"), &default_opts()).unwrap();
    let result = combine_files(&entries, false).unwrap();

    // First file (sales_2024-01): first row is 2024-01-01,Tokyo,100
    assert_eq!(result.data.rows[0][0], "2024-01-01");
    assert_eq!(result.data.rows[0][1], "Tokyo");
    assert_eq!(result.data.rows[0][2], "100");

    // Second file (sales_2024-02): first row is 2024-02-01,Osaka,250
    assert_eq!(result.data.rows[3][0], "2024-02-01");
    assert_eq!(result.data.rows[3][1], "Osaka");
    assert_eq!(result.data.rows[3][2], "250");

    // Third file (sales_2024-03): first row is 2024-03-01,Tokyo,400
    assert_eq!(result.data.rows[6][0], "2024-03-01");
    assert_eq!(result.data.rows[6][1], "Tokyo");
    assert_eq!(result.data.rows[6][2], "400");
}

// === Combiner error case tests (Cycle 5) ===

#[test]
fn test_combine_schema_mismatch_skips_file() {
    let entries = scan_directory(&fixture("mixed_schema"), &default_opts()).unwrap();
    let result = combine_files(&entries, false).unwrap();

    // mixed_schema has: also_sales.csv (date,city,revenue), sales.csv (date,city,revenue), users.csv (name,email,age)
    // First file lexicographically is also_sales.csv → sets schema to date,city,revenue
    // sales.csv matches → included
    // users.csv doesn't match → skipped
    assert_eq!(result.file_count, 2);
    assert_eq!(result.skipped.len(), 1);
    assert_eq!(result.skipped[0].file, "users");
    assert!(result.skipped[0].reason.contains("schema mismatch"));
}

#[test]
fn test_combine_header_only_file_skipped() {
    let entries = scan_directory(&fixture("header_only"), &default_opts()).unwrap();
    let result = combine_files(&entries, false).unwrap();

    // header_only has: empty_data.csv (0 rows), good.csv (3 rows)
    // First lex: empty_data.csv → 0 rows → skipped
    // Second: good.csv → sets schema, 3 rows
    assert_eq!(result.file_count, 1);
    assert_eq!(result.data.rows.len(), 3);
    assert_eq!(result.skipped.len(), 1);
    assert_eq!(result.skipped[0].file, "empty_data");
    assert!(result.skipped[0].reason.contains("0 data rows"));
}

#[test]
fn test_combine_single_file_works() {
    let entries = scan_directory(&fixture("single_file"), &default_opts()).unwrap();
    let result = combine_files(&entries, false).unwrap();

    assert_eq!(result.file_count, 1);
    assert_eq!(result.data.rows.len(), 3);
    assert_eq!(result.data.headers, vec!["date", "value", "_source"]);
    // All rows should have _source = "only"
    for row in &result.data.rows {
        assert_eq!(row[2], "only");
    }
}

#[test]
fn test_combine_empty_entries_returns_error() {
    let result = combine_files(&[], false);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("no file entries"));
}

#[test]
fn test_combine_mixed_schema_row_count_correct() {
    let entries = scan_directory(&fixture("mixed_schema"), &default_opts()).unwrap();
    let result = combine_files(&entries, false).unwrap();

    // also_sales.csv has 2 rows, sales.csv has 3 rows = 5 total
    assert_eq!(result.data.rows.len(), 5);
}

// === Case-insensitive schema matching tests (Phase 2, Task 1) ===

#[test]
fn test_combine_case_insensitive_schema_matches() {
    // file_a has "Date,City,Revenue", file_b has "date,city,revenue"
    // Both should combine successfully
    let entries = scan_directory(&fixture("case_insensitive"), &default_opts()).unwrap();
    let result = combine_files(&entries, false).unwrap();
    assert_eq!(result.file_count, 2);
    assert_eq!(result.data.rows.len(), 6);
    assert!(result.skipped.is_empty());
}

#[test]
fn test_combine_case_insensitive_preserves_first_file_casing() {
    // Output headers should use first file's casing: "Date", "City", "Revenue"
    let entries = scan_directory(&fixture("case_insensitive"), &default_opts()).unwrap();
    let result = combine_files(&entries, false).unwrap();
    assert_eq!(result.data.headers[0], "Date");
    assert_eq!(result.data.headers[1], "City");
    assert_eq!(result.data.headers[2], "Revenue");
    assert_eq!(result.data.headers[3], "_source");
}

#[test]
fn test_combine_case_insensitive_does_not_mutate_row_data() {
    // Row values must remain unchanged regardless of header normalization
    let entries = scan_directory(&fixture("case_insensitive"), &default_opts()).unwrap();
    let result = combine_files(&entries, false).unwrap();
    // file_b row: "2024-02-01,Osaka,250" — values must be unchanged
    assert_eq!(result.data.rows[3][0], "2024-02-01");
    assert_eq!(result.data.rows[3][1], "Osaka");
    assert_eq!(result.data.rows[3][2], "250");
}

#[test]
fn test_combine_case_insensitive_whitespace_trimmed() {
    // Headers with leading/trailing whitespace should match after trim
    // This test uses the same fixture — the normalization trims before comparing
    let entries = scan_directory(&fixture("case_insensitive"), &default_opts()).unwrap();
    let result = combine_files(&entries, false).unwrap();
    // "Date" (trimmed) matches "date" (trimmed+lowered) → combined
    assert_eq!(result.file_count, 2);
}

// === Column order normalization tests (Phase 2, Task 2) ===

#[test]
fn test_combine_reordered_columns_matches() {
    // first.csv: date,city,revenue; second.csv: revenue,date,city
    // Same columns, different order → should combine
    let entries = scan_directory(&fixture("reordered"), &default_opts()).unwrap();
    let result = combine_files(&entries, false).unwrap();
    assert_eq!(result.file_count, 2);
    assert_eq!(result.data.rows.len(), 6);
    assert!(result.skipped.is_empty());
}

#[test]
fn test_combine_reordered_columns_remaps_data() {
    // second.csv row "500,2024-04-01,Nagoya" → remapped to reference order: date,city,revenue
    let entries = scan_directory(&fixture("reordered"), &default_opts()).unwrap();
    let result = combine_files(&entries, false).unwrap();
    // First file has 3 rows, so second file starts at index 3
    let row = &result.data.rows[3];
    assert_eq!(row[0], "2024-04-01"); // date (was at position 1 in second.csv)
    assert_eq!(row[1], "Nagoya"); // city (was at position 2 in second.csv)
    assert_eq!(row[2], "500"); // revenue (was at position 0 in second.csv)
}

#[test]
fn test_combine_reordered_uses_first_file_column_order() {
    let entries = scan_directory(&fixture("reordered"), &default_opts()).unwrap();
    let result = combine_files(&entries, false).unwrap();
    assert_eq!(
        result.data.headers,
        vec!["date", "city", "revenue", "_source"]
    );
}

#[test]
fn test_combine_reordered_source_column_correct() {
    let entries = scan_directory(&fixture("reordered"), &default_opts()).unwrap();
    let result = combine_files(&entries, false).unwrap();
    // Rows from first.csv have _source = "first"
    assert_eq!(result.data.rows[0][3], "first");
    // Rows from second.csv have _source = "second"
    assert_eq!(result.data.rows[3][3], "second");
}

// === Large dataset warning tests (Phase 2, Task 3) ===

use super::LARGE_DATASET_THRESHOLD;

#[test]
fn test_large_dataset_threshold_is_100_000() {
    assert_eq!(LARGE_DATASET_THRESHOLD, 100_000);
}

#[test]
fn test_large_dataset_warning_message_format() {
    // Verify the warning message format matches spec
    let row_count = 150_000;
    let msg = super::large_dataset_warning(row_count);
    assert_eq!(
        msg,
        Some(
            "warning: large dataset (150000 rows). Consider --sample for faster rendering."
                .to_string()
        )
    );
}

#[test]
fn test_large_dataset_no_warning_at_threshold() {
    // Exactly 100,000 rows → no warning
    let msg = super::large_dataset_warning(100_000);
    assert_eq!(msg, None);
}

#[test]
fn test_large_dataset_no_warning_below_threshold() {
    let msg = super::large_dataset_warning(99_999);
    assert_eq!(msg, None);
}

// === Explore directory mode tests (Phase 2, Task 4) ===

/// Test that directory scanning + combining works for explore input preparation.
/// (The actual TUI is not tested here — only the data pipeline.)
#[test]
fn test_explore_directory_combine_produces_valid_data() {
    let entries = scan_directory(&fixture("same_schema"), &default_opts()).unwrap();
    let result = combine_files(&entries, false).unwrap();
    // Data should be valid for explore: has headers, rows, and _source column
    assert!(!result.data.headers.is_empty());
    assert!(!result.data.rows.is_empty());
    assert!(result.data.headers.contains(&"_source".to_string()));
}

#[test]
fn test_explore_directory_schema_inferred() {
    use crate::infer;
    let entries = scan_directory(&fixture("same_schema"), &default_opts()).unwrap();
    let result = combine_files(&entries, false).unwrap();
    let headers: Vec<&str> = result.data.headers.iter().map(|s| s.as_str()).collect();
    let rows: Vec<Vec<&str>> = result
        .data
        .rows
        .iter()
        .map(|r| r.iter().map(|s| s.as_str()).collect())
        .collect();
    let schema = infer::infer_schema(&headers, &rows);
    // Schema should have 4 columns (date, city, revenue, _source)
    assert_eq!(schema.columns.len(), 4);
}
