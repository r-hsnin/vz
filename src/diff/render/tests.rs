use std::path::Path;

use super::html::{print_diff_html, print_diff_line_html};
use super::json::print_diff_line_json;
use super::markdown::{print_diff_line_markdown, print_diff_markdown};
use super::spark::{print_diff_line_spark, print_diff_spark};
use crate::chart::selector::SortOrder;
use crate::cli::DiffParams;
use crate::diff::{DiffEntry, DiffResult, DiffTimeSeries};

fn diff_params(sort: Option<SortOrder>, limit: Option<usize>) -> DiffParams {
    DiffParams {
        query: crate::chart::Query::default(),
        no_header: false,
        format: None,
        output: None,
        sort,
        limit,
        width: None,
        height: None,
        title: None,
        theme: None,
    }
}

fn sample_entries() -> Vec<DiffEntry> {
    vec![
        DiffEntry {
            label: "Tokyo".into(),
            before: 1000.0,
            after: 1200.0,
            delta: 200.0,
            pct_change: Some(20.0),
        },
        DiffEntry {
            label: "Osaka".into(),
            before: 1500.0,
            after: 1350.0,
            delta: -150.0,
            pct_change: Some(-10.0),
        },
        DiffEntry {
            label: "Nagoya".into(),
            before: 800.0,
            after: 950.0,
            delta: 150.0,
            pct_change: Some(18.75),
        },
    ]
}

#[test]
fn test_diff_spark_format() {
    let diff = DiffResult {
        entries: sample_entries(),
        x_column: "city".into(),
        y_column: "revenue".into(),
        before_rows: 3,
        after_rows: 3,
        overall_pct: Some(6.06),
    };
    // Just verify it doesn't panic; output format tested via integration tests
    print_diff_spark(&diff);
}

#[test]
fn test_build_diff_bar_data_sort_desc() {
    let entries = sample_entries();
    let tuples: Vec<(String, f64, Option<f64>, f64)> = entries
        .iter()
        .map(|e| (e.label.clone(), e.after, e.pct_change, e.delta))
        .collect();
    let data = crate::chart::data_builder::build_diff_bar_data(
        &tuples,
        Some(SortOrder::Desc),
        None,
        "revenue".into(),
        None,
    );
    assert_eq!(data.labels[0], "Tokyo ▲ +20%"); // delta +200
    assert_eq!(data.labels[1], "Nagoya ▲ +19%"); // delta +150
    assert_eq!(data.labels[2], "Osaka ▼ -10%"); // delta -150
}

#[test]
fn test_build_diff_bar_data_sort_asc() {
    let entries = sample_entries();
    let tuples: Vec<(String, f64, Option<f64>, f64)> = entries
        .iter()
        .map(|e| (e.label.clone(), e.after, e.pct_change, e.delta))
        .collect();
    let data = crate::chart::data_builder::build_diff_bar_data(
        &tuples,
        Some(SortOrder::Asc),
        None,
        "revenue".into(),
        None,
    );
    assert!(data.labels[0].starts_with("Osaka")); // delta -150
}

#[test]
fn test_build_diff_bar_data_top_limit() {
    let entries = sample_entries();
    let tuples: Vec<(String, f64, Option<f64>, f64)> = entries
        .iter()
        .map(|e| (e.label.clone(), e.after, e.pct_change, e.delta))
        .collect();
    let data = crate::chart::data_builder::build_diff_bar_data(
        &tuples,
        Some(SortOrder::Desc),
        Some(2),
        "revenue".into(),
        None,
    );
    assert_eq!(data.labels.len(), 2);
    assert_eq!(data.labels[0], "Tokyo ▲ +20%"); // highest delta
}

// --- render_diff_line tests ---

fn sample_ts() -> DiffTimeSeries {
    DiffTimeSeries {
        before: vec![(0.0, 100.0), (1.0, 120.0), (2.0, 140.0)],
        after: vec![(0.0, 110.0), (1.0, 130.0), (2.0, 150.0)],
        x_labels: vec![
            "2024-01-01".into(),
            "2024-01-02".into(),
            "2024-01-03".into(),
        ],
        x_column: "date".into(),
        y_column: "revenue".into(),
        before_rows: 3,
        after_rows: 3,
        overall_pct: Some(8.33),
    }
}

#[test]
fn test_diff_line_spark_format() {
    let ts = sample_ts();
    // Just verify it doesn't panic
    print_diff_line_spark(&ts, Path::new("before.csv"), Path::new("after.csv"));
}

#[test]
fn test_diff_line_json_structure() {
    let ts = sample_ts();
    // Verify the JSON function succeeds
    let result = print_diff_line_json(&ts, Path::new("before.csv"), Path::new("after.csv"));
    assert!(result.is_ok());
}

// --- Markdown output tests ---

#[test]
fn test_diff_markdown_categorical_basic() {
    let diff = DiffResult {
        entries: sample_entries(),
        x_column: "city".into(),
        y_column: "revenue".into(),
        before_rows: 3,
        after_rows: 3,
        overall_pct: Some(6.06),
    };
    let params = diff_params(None, None);
    // Verify it doesn't panic and produces output
    print_diff_markdown(
        &params,
        &diff,
        Path::new("before.csv"),
        Path::new("after.csv"),
    );
}

#[test]
fn test_diff_markdown_categorical_with_sort() {
    let diff = DiffResult {
        entries: sample_entries(),
        x_column: "city".into(),
        y_column: "revenue".into(),
        before_rows: 3,
        after_rows: 3,
        overall_pct: Some(6.06),
    };
    let params = diff_params(Some(SortOrder::Desc), None);
    print_diff_markdown(
        &params,
        &diff,
        Path::new("before.csv"),
        Path::new("after.csv"),
    );
}

#[test]
fn test_diff_markdown_categorical_with_top() {
    let diff = DiffResult {
        entries: sample_entries(),
        x_column: "city".into(),
        y_column: "revenue".into(),
        before_rows: 3,
        after_rows: 3,
        overall_pct: Some(6.06),
    };
    let params = diff_params(Some(SortOrder::Desc), Some(2));
    print_diff_markdown(
        &params,
        &diff,
        Path::new("before.csv"),
        Path::new("after.csv"),
    );
}

#[test]
fn test_diff_markdown_temporal_basic() {
    let ts = sample_ts();
    // Verify it doesn't panic
    print_diff_line_markdown(&ts, Path::new("before.csv"), Path::new("after.csv"));
}

#[test]
fn test_diff_markdown_no_overall_pct() {
    let diff = DiffResult {
        entries: sample_entries(),
        x_column: "city".into(),
        y_column: "revenue".into(),
        before_rows: 3,
        after_rows: 3,
        overall_pct: None,
    };
    let params = diff_params(None, None);
    print_diff_markdown(
        &params,
        &diff,
        Path::new("before.csv"),
        Path::new("after.csv"),
    );
}

// --- HTML output tests ---

#[test]
fn test_diff_html_categorical_basic() {
    let diff = DiffResult {
        entries: sample_entries(),
        x_column: "city".into(),
        y_column: "revenue".into(),
        before_rows: 3,
        after_rows: 3,
        overall_pct: Some(6.06),
    };
    let params = diff_params(None, None);
    // Verify it doesn't panic (output goes to stdout)
    print_diff_html(
        &params,
        &diff,
        Path::new("before.csv"),
        Path::new("after.csv"),
    );
}

#[test]
fn test_diff_html_categorical_with_sort() {
    let diff = DiffResult {
        entries: sample_entries(),
        x_column: "city".into(),
        y_column: "revenue".into(),
        before_rows: 3,
        after_rows: 3,
        overall_pct: Some(6.06),
    };
    let params = diff_params(Some(SortOrder::Desc), None);
    print_diff_html(
        &params,
        &diff,
        Path::new("before.csv"),
        Path::new("after.csv"),
    );
}

#[test]
fn test_diff_html_categorical_with_top() {
    let diff = DiffResult {
        entries: sample_entries(),
        x_column: "city".into(),
        y_column: "revenue".into(),
        before_rows: 3,
        after_rows: 3,
        overall_pct: Some(6.06),
    };
    let params = diff_params(Some(SortOrder::Desc), Some(2));
    print_diff_html(
        &params,
        &diff,
        Path::new("before.csv"),
        Path::new("after.csv"),
    );
}

#[test]
fn test_diff_html_temporal_basic() {
    let ts = sample_ts();
    let params = diff_params(None, None);
    print_diff_line_html(
        &params,
        &ts,
        Path::new("before.csv"),
        Path::new("after.csv"),
    );
}

#[test]
fn test_diff_html_no_overall_pct() {
    let diff = DiffResult {
        entries: sample_entries(),
        x_column: "city".into(),
        y_column: "revenue".into(),
        before_rows: 3,
        after_rows: 3,
        overall_pct: None,
    };
    let params = diff_params(None, None);
    print_diff_html(
        &params,
        &diff,
        Path::new("before.csv"),
        Path::new("after.csv"),
    );
}
