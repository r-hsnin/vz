use std::path::{Path, PathBuf};

use clap::Parser;

use crate::cli::Cli;
use crate::diff::compute::{compute_diff, compute_diff_temporal};
use crate::diff::schema::{col_index, validate_schema};
use crate::loader::LoadedData;

fn make_data(headers: &[&str], rows: &[&[&str]]) -> LoadedData {
    LoadedData {
        headers: headers.iter().map(|s| s.to_string()).collect(),
        rows: rows
            .iter()
            .map(|r| r.iter().map(|s| s.to_string()).collect())
            .collect(),
    }
}

#[test]
fn test_validate_schema_same_columns() {
    let a = make_data(&["city", "revenue"], &[&["Tokyo", "100"]]);
    let b = make_data(&["city", "revenue"], &[&["Tokyo", "200"]]);
    let r = validate_schema(&a, &b, Path::new("a.csv"), Path::new("b.csv"));
    assert!(r.is_ok());
}

#[test]
fn test_validate_schema_case_insensitive() {
    let a = make_data(&["City", "Revenue"], &[&["Tokyo", "100"]]);
    let b = make_data(&["city", "revenue"], &[&["Tokyo", "200"]]);
    let r = validate_schema(&a, &b, Path::new("a.csv"), Path::new("b.csv"));
    assert!(r.is_ok());
}

#[test]
fn test_validate_schema_different_column_count() {
    let a = make_data(&["city", "revenue"], &[&["Tokyo", "100"]]);
    let b = make_data(&["city", "revenue", "profit"], &[&["Tokyo", "200", "50"]]);
    let r = validate_schema(&a, &b, Path::new("a.csv"), Path::new("b.csv"));
    assert!(r.is_err());
    assert!(format!("{}", r.unwrap_err()).contains("Schema mismatch"));
}

#[test]
fn test_validate_schema_different_column_names() {
    let a = make_data(&["city", "revenue"], &[&["Tokyo", "100"]]);
    let b = make_data(&["product", "cost"], &[&["Widget", "50"]]);
    let r = validate_schema(&a, &b, Path::new("a.csv"), Path::new("b.csv"));
    assert!(r.is_err());
}

#[test]
fn test_compute_diff_basic_increase() {
    let a = make_data(
        &["city", "revenue"],
        &[&["Tokyo", "1000"], &["Osaka", "1500"]],
    );
    let b = make_data(
        &["city", "revenue"],
        &[&["Tokyo", "1200"], &["Osaka", "1350"]],
    );
    let diff = compute_diff(&a, &b, "city", "revenue").unwrap();
    assert_eq!(diff.entries.len(), 2);

    let tokyo = &diff.entries[0];
    assert_eq!(tokyo.label, "Tokyo");
    assert_eq!(tokyo.before, 1000.0);
    assert_eq!(tokyo.after, 1200.0);
    assert_eq!(tokyo.delta, 200.0);
    assert!((tokyo.pct_change.unwrap() - 20.0).abs() < 0.01);

    let osaka = &diff.entries[1];
    assert_eq!(osaka.label, "Osaka");
    assert_eq!(osaka.before, 1500.0);
    assert_eq!(osaka.after, 1350.0);
    assert_eq!(osaka.delta, -150.0);
    assert!((osaka.pct_change.unwrap() - (-10.0)).abs() < 0.01);
}

#[test]
fn test_compute_diff_no_change() {
    let a = make_data(&["city", "revenue"], &[&["Tokyo", "500"]]);
    let b = make_data(&["city", "revenue"], &[&["Tokyo", "500"]]);
    let diff = compute_diff(&a, &b, "city", "revenue").unwrap();
    assert_eq!(diff.entries[0].delta, 0.0);
    assert_eq!(diff.entries[0].pct_change, Some(0.0));
}

#[test]
fn test_compute_diff_from_zero() {
    let a = make_data(&["city", "revenue"], &[&["Tokyo", "0"]]);
    let b = make_data(&["city", "revenue"], &[&["Tokyo", "500"]]);
    let diff = compute_diff(&a, &b, "city", "revenue").unwrap();
    // From zero: no percentage (shown as "new")
    assert_eq!(diff.entries[0].delta, 500.0);
    assert_eq!(diff.entries[0].pct_change, None);
}

#[test]
fn test_compute_diff_new_category_in_after() {
    let a = make_data(&["city", "revenue"], &[&["Tokyo", "1000"]]);
    let b = make_data(
        &["city", "revenue"],
        &[&["Tokyo", "1200"], &["Osaka", "800"]],
    );
    let diff = compute_diff(&a, &b, "city", "revenue").unwrap();
    assert_eq!(diff.entries.len(), 2);
    let osaka = &diff.entries[1];
    assert_eq!(osaka.label, "Osaka");
    assert_eq!(osaka.before, 0.0);
    assert_eq!(osaka.after, 800.0);
    assert_eq!(osaka.delta, 800.0);
    assert_eq!(osaka.pct_change, None); // from zero
}

#[test]
fn test_compute_diff_removed_category() {
    let a = make_data(
        &["city", "revenue"],
        &[&["Tokyo", "1000"], &["Osaka", "800"]],
    );
    let b = make_data(&["city", "revenue"], &[&["Tokyo", "1200"]]);
    let diff = compute_diff(&a, &b, "city", "revenue").unwrap();
    assert_eq!(diff.entries.len(), 2);
    let osaka = &diff.entries[1];
    assert_eq!(osaka.label, "Osaka");
    assert_eq!(osaka.before, 800.0);
    assert_eq!(osaka.after, 0.0);
    assert_eq!(osaka.delta, -800.0);
    assert!((osaka.pct_change.unwrap() - (-100.0)).abs() < 0.01);
}

#[test]
fn test_compute_diff_aggregates_duplicates() {
    let a = make_data(
        &["city", "revenue"],
        &[&["Tokyo", "500"], &["Tokyo", "500"], &["Osaka", "300"]],
    );
    let b = make_data(
        &["city", "revenue"],
        &[&["Tokyo", "600"], &["Tokyo", "600"], &["Osaka", "400"]],
    );
    let diff = compute_diff(&a, &b, "city", "revenue").unwrap();
    let tokyo = &diff.entries[0];
    assert_eq!(tokyo.before, 1000.0); // 500+500
    assert_eq!(tokyo.after, 1200.0); // 600+600
}

#[test]
fn test_compute_diff_overall_pct() {
    let a = make_data(
        &["city", "revenue"],
        &[&["Tokyo", "1000"], &["Osaka", "1500"]],
    );
    let b = make_data(
        &["city", "revenue"],
        &[&["Tokyo", "1200"], &["Osaka", "1350"]],
    );
    let diff = compute_diff(&a, &b, "city", "revenue").unwrap();
    // total before: 2500, total after: 2550, change: +50/2500 = +2%
    assert!((diff.overall_pct.unwrap() - 2.0).abs() < 0.01);
}

#[test]
fn test_col_index_case_insensitive_fallback() {
    let headers: Vec<String> = vec!["City".to_string(), "Revenue".to_string()];
    assert_eq!(col_index(&headers, "city"), Some(0));
    assert_eq!(col_index(&headers, "Revenue"), Some(1));
    assert_eq!(col_index(&headers, "missing"), None);
}

#[test]
fn test_diff_pair_two_positional() {
    let cli = Cli::try_parse_from(["vz", "a.csv", "b.csv"]).unwrap();
    let pair = cli.diff_pair();
    assert_eq!(pair, Some((PathBuf::from("a.csv"), PathBuf::from("b.csv"))));
}

#[test]
fn test_diff_pair_with_flag() {
    let cli = Cli::try_parse_from(["vz", "a.csv", "--diff", "b.csv"]).unwrap();
    let pair = cli.diff_pair();
    assert_eq!(pair, Some((PathBuf::from("a.csv"), PathBuf::from("b.csv"))));
}

#[test]
fn test_diff_pair_single_file_no_diff() {
    let cli = Cli::try_parse_from(["vz", "a.csv"]).unwrap();
    assert_eq!(cli.diff_pair(), None);
}

// --- compute_diff_temporal tests ---

#[test]
fn test_compute_diff_temporal_basic() {
    let before = make_data(
        &["date", "revenue"],
        &[
            &["2024-01-01", "100"],
            &["2024-01-02", "120"],
            &["2024-01-03", "140"],
        ],
    );
    let after = make_data(
        &["date", "revenue"],
        &[
            &["2024-01-01", "110"],
            &["2024-01-02", "130"],
            &["2024-01-03", "150"],
        ],
    );
    let ts = compute_diff_temporal(&before, &after, "date", "revenue").unwrap();
    assert_eq!(ts.before.len(), 3);
    assert_eq!(ts.after.len(), 3);
    assert_eq!(ts.x_labels.len(), 3);
    assert_eq!(ts.x_labels[0], "2024-01-01");
    assert_eq!(ts.before[0], (0.0, 100.0));
    assert_eq!(ts.after[0], (0.0, 110.0));
    assert_eq!(ts.before[2], (2.0, 140.0));
    assert_eq!(ts.after[2], (2.0, 150.0));
}

#[test]
fn test_compute_diff_temporal_sorted_dates() {
    // Verify dates are sorted even if input is unordered
    let before = make_data(
        &["date", "revenue"],
        &[&["2024-01-03", "140"], &["2024-01-01", "100"]],
    );
    let after = make_data(
        &["date", "revenue"],
        &[&["2024-01-02", "130"], &["2024-01-01", "110"]],
    );
    let ts = compute_diff_temporal(&before, &after, "date", "revenue").unwrap();
    assert_eq!(ts.x_labels, vec!["2024-01-01", "2024-01-02", "2024-01-03"]);
}

#[test]
fn test_compute_diff_temporal_non_overlapping() {
    // Before has dates not in after, and vice versa
    let before = make_data(
        &["date", "revenue"],
        &[&["2024-01-01", "100"], &["2024-01-02", "120"]],
    );
    let after = make_data(
        &["date", "revenue"],
        &[&["2024-01-02", "130"], &["2024-01-03", "150"]],
    );
    let ts = compute_diff_temporal(&before, &after, "date", "revenue").unwrap();
    // Union: 01, 02, 03
    assert_eq!(ts.x_labels.len(), 3);
    // Before has points at index 0 and 1
    assert_eq!(ts.before.len(), 2);
    assert_eq!(ts.before[0], (0.0, 100.0)); // 2024-01-01
    assert_eq!(ts.before[1], (1.0, 120.0)); // 2024-01-02
    // After has points at index 1 and 2
    assert_eq!(ts.after.len(), 2);
    assert_eq!(ts.after[0], (1.0, 130.0)); // 2024-01-02
    assert_eq!(ts.after[1], (2.0, 150.0)); // 2024-01-03
}

#[test]
fn test_compute_diff_temporal_aggregates_duplicates() {
    let before = make_data(
        &["date", "revenue"],
        &[
            &["2024-01-01", "50"],
            &["2024-01-01", "50"],
            &["2024-01-02", "120"],
        ],
    );
    let after = make_data(
        &["date", "revenue"],
        &[&["2024-01-01", "110"], &["2024-01-02", "130"]],
    );
    let ts = compute_diff_temporal(&before, &after, "date", "revenue").unwrap();
    assert_eq!(ts.before[0], (0.0, 100.0)); // 50+50
    assert_eq!(ts.after[0], (0.0, 110.0));
}

#[test]
fn test_compute_diff_temporal_overall_pct() {
    let before = make_data(
        &["date", "revenue"],
        &[&["2024-01-01", "100"], &["2024-01-02", "200"]],
    );
    let after = make_data(
        &["date", "revenue"],
        &[&["2024-01-01", "120"], &["2024-01-02", "240"]],
    );
    let ts = compute_diff_temporal(&before, &after, "date", "revenue").unwrap();
    // Before sum: 300, After sum: 360 → +20%
    assert!((ts.overall_pct.unwrap() - 20.0).abs() < 0.01);
}

#[test]
fn test_compute_diff_temporal_single_point() {
    let before = make_data(&["date", "revenue"], &[&["2024-01-01", "100"]]);
    let after = make_data(&["date", "revenue"], &[&["2024-01-01", "200"]]);
    let ts = compute_diff_temporal(&before, &after, "date", "revenue").unwrap();
    assert_eq!(ts.before.len(), 1);
    assert_eq!(ts.after.len(), 1);
    assert_eq!(ts.x_labels, vec!["2024-01-01"]);
    assert!((ts.overall_pct.unwrap() - 100.0).abs() < 0.01);
}
