//! Unit tests for auto-sampling in directory mode.

use super::{MAX_COMBINED_ROWS, auto_sample_combined};
use crate::loader::LoadedData;

/// Generate test data with N rows for auto-sampling tests.
fn make_auto_sample_data(n: usize) -> LoadedData {
    let headers = vec![
        "id".to_string(),
        "value".to_string(),
        "_source".to_string(),
        "_file_date".to_string(),
    ];
    let rows: Vec<Vec<String>> = (0..n)
        .map(|i| {
            vec![
                i.to_string(),
                (i * 10).to_string(),
                format!("file_{}", i % 3),
                "2024-01-01".to_string(),
            ]
        })
        .collect();
    LoadedData { headers, rows }
}

#[test]
fn test_max_combined_rows_constant() {
    assert_eq!(MAX_COMBINED_ROWS, 1_000_000);
}

#[test]
fn test_auto_sample_reduces_to_limit() {
    let data = make_auto_sample_data(100);
    let (result, warning) = auto_sample_combined(data, 20, false);
    assert_eq!(result.rows.len(), 20);
    assert!(warning.is_some());
}

#[test]
fn test_auto_sample_no_reduction_at_limit() {
    let data = make_auto_sample_data(20);
    let (result, warning) = auto_sample_combined(data, 20, false);
    assert_eq!(result.rows.len(), 20);
    assert!(warning.is_none());
}

#[test]
fn test_auto_sample_no_reduction_below_limit() {
    let data = make_auto_sample_data(15);
    let (result, warning) = auto_sample_combined(data, 20, false);
    assert_eq!(result.rows.len(), 15);
    assert!(warning.is_none());
}

#[test]
fn test_auto_sample_one_over_limit() {
    let data = make_auto_sample_data(21);
    let (result, warning) = auto_sample_combined(data, 20, false);
    assert_eq!(result.rows.len(), 20);
    assert!(warning.is_some());
}

#[test]
fn test_auto_sample_warning_message_format() {
    let data = make_auto_sample_data(50);
    let (_, warning) = auto_sample_combined(data, 20, false);
    let msg = warning.unwrap();
    assert!(msg.contains("exceeded 20 rows"), "got: {msg}");
    assert!(msg.contains("auto-sampled to 20 rows"), "got: {msg}");
}

#[test]
fn test_auto_sample_no_limit_flag_bypasses() {
    let data = make_auto_sample_data(100);
    let (result, warning) = auto_sample_combined(data, 20, true);
    assert_eq!(result.rows.len(), 100);
    assert!(warning.is_none());
}

#[test]
fn test_auto_sample_preserves_headers() {
    let data = make_auto_sample_data(50);
    let original_headers = data.headers.clone();
    let (result, _) = auto_sample_combined(data, 20, false);
    assert_eq!(result.headers, original_headers);
}

#[test]
fn test_auto_sample_source_distribution() {
    // Create data with 3 source files (30 rows each = 90 total), limit to 30
    let mut rows = Vec::new();
    let headers = vec![
        "id".into(),
        "value".into(),
        "_source".into(),
        "_file_date".into(),
    ];
    for source in &["file_a", "file_b", "file_c"] {
        for i in 0..30 {
            rows.push(vec![
                i.to_string(),
                (i * 2).to_string(),
                source.to_string(),
                String::new(),
            ]);
        }
    }
    let data = LoadedData { headers, rows };
    let (result, _) = auto_sample_combined(data, 30, false);

    // All 3 sources should appear in sampled output (systematic sampling preserves distribution)
    let source_idx = 2;
    let sources: std::collections::HashSet<&str> =
        result.rows.iter().map(|r| r[source_idx].as_str()).collect();
    assert!(sources.contains("file_a"), "file_a missing from sample");
    assert!(sources.contains("file_b"), "file_b missing from sample");
    assert!(sources.contains("file_c"), "file_c missing from sample");
}

#[test]
fn test_auto_sample_systematic_picks_evenly() {
    // 100 rows (ids 0-99), limit to 10 → should pick every 10th
    let data = make_auto_sample_data(100);
    let (result, _) = auto_sample_combined(data, 10, false);
    assert_eq!(result.rows.len(), 10);
    // First row should be id=0, last should be id=90
    assert_eq!(result.rows[0][0], "0");
    assert_eq!(result.rows[9][0], "90");
}
