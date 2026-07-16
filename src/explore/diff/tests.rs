//! Unit tests for DiffExploreApp.

use crossterm::event::KeyCode;

use crate::cli::SortOrder;
use crate::diff::{DiffEntry, DiffResult, DiffTimeSeries};

use super::*;

fn make_categorical_app() -> DiffExploreApp {
    let result = DiffResult {
        entries: vec![
            DiffEntry {
                label: "Tokyo".to_string(),
                before: 100.0,
                after: 150.0,
                delta: 50.0,
                pct_change: Some(50.0),
            },
            DiffEntry {
                label: "Osaka".to_string(),
                before: 200.0,
                after: 180.0,
                delta: -20.0,
                pct_change: Some(-10.0),
            },
            DiffEntry {
                label: "Nagoya".to_string(),
                before: 80.0,
                after: 80.0,
                delta: 0.0,
                pct_change: Some(0.0),
            },
        ],
        x_column: "city".to_string(),
        y_column: "revenue".to_string(),
        before_rows: 3,
        after_rows: 3,
        overall_pct: Some(7.9),
    };
    DiffExploreApp::new(
        DiffData::Categorical(result),
        "before.csv".to_string(),
        "after.csv".to_string(),
        crate::theme::Theme::dark(),
    )
}

fn make_temporal_app() -> DiffExploreApp {
    let ts = DiffTimeSeries {
        before: vec![(0.0, 100.0), (1.0, 200.0), (2.0, 150.0)],
        after: vec![(0.0, 120.0), (1.0, 250.0), (2.0, 180.0)],
        x_labels: vec![
            "2024-01".to_string(),
            "2024-02".to_string(),
            "2024-03".to_string(),
        ],
        x_column: "date".to_string(),
        y_column: "revenue".to_string(),
        before_rows: 3,
        after_rows: 3,
        overall_pct: Some(22.2),
    };
    DiffExploreApp::new(
        DiffData::Temporal(ts),
        "ts_before.csv".to_string(),
        "ts_after.csv".to_string(),
        crate::theme::Theme::dark(),
    )
}

#[test]
fn test_initial_state_categorical() {
    let app = make_categorical_app();
    assert_eq!(app.view_mode, ViewMode::Chart);
    assert!(!app.should_quit);
    assert!(!app.show_help);
    assert!(app.status_message.is_some());
    assert_eq!(app.table_offset, 0);
    assert!(app.sort_order.is_none());
    assert!(!app.is_temporal());
    assert_eq!(app.entry_count(), 3);
}

#[test]
fn test_initial_state_temporal() {
    let app = make_temporal_app();
    assert!(app.is_temporal());
    assert_eq!(app.entry_count(), 3);
    assert_eq!(app.x_column(), "date");
    assert_eq!(app.y_column(), "revenue");
}

#[test]
fn test_quit_key() {
    let mut app = make_categorical_app();
    app.handle_key(KeyCode::Char('q'));
    assert!(app.should_quit);
}

#[test]
fn test_esc_key() {
    let mut app = make_categorical_app();
    app.handle_key(KeyCode::Esc);
    assert!(app.should_quit);
}

#[test]
fn test_toggle_view_mode() {
    let mut app = make_categorical_app();
    assert_eq!(app.view_mode, ViewMode::Chart);
    app.handle_key(KeyCode::Char('d'));
    assert_eq!(app.view_mode, ViewMode::Table);
    app.handle_key(KeyCode::Tab);
    assert_eq!(app.view_mode, ViewMode::Chart);
}

#[test]
fn test_help_overlay() {
    let mut app = make_categorical_app();
    app.handle_key(KeyCode::Char('?'));
    assert!(app.show_help);
    // Any key dismisses help
    app.handle_key(KeyCode::Char('x'));
    assert!(!app.show_help);
}

#[test]
fn test_sort_cycle() {
    let mut app = make_categorical_app();
    assert!(app.sort_order.is_none());
    app.handle_key(KeyCode::Char('s'));
    assert_eq!(app.sort_order, Some(SortOrder::Desc));
    app.handle_key(KeyCode::Char('s'));
    assert_eq!(app.sort_order, Some(SortOrder::Asc));
    app.handle_key(KeyCode::Char('s'));
    assert!(app.sort_order.is_none());
}

#[test]
fn test_scroll_table() {
    let mut app = make_categorical_app();
    app.handle_key(KeyCode::Char('j'));
    assert_eq!(app.table_offset, 1);
    app.handle_key(KeyCode::Char('k'));
    assert_eq!(app.table_offset, 0);
    // Don't go below 0
    app.handle_key(KeyCode::Char('k'));
    assert_eq!(app.table_offset, 0);
}

#[test]
fn test_scroll_jump_to_end() {
    let mut app = make_categorical_app();
    app.handle_key(KeyCode::Char('G'));
    assert_eq!(app.table_offset, 2); // 3 entries, max=2
    app.handle_key(KeyCode::Char('g'));
    assert_eq!(app.table_offset, 0);
}

#[test]
fn test_na_keys_in_diff_mode() {
    let mut app = make_categorical_app();
    app.status_message = None;
    app.handle_key(KeyCode::Char('h'));
    assert_eq!(app.status_message.as_deref(), Some("N/A in diff mode"));

    app.status_message = None;
    app.handle_key(KeyCode::Char('c'));
    assert_eq!(app.status_message.as_deref(), Some("N/A in diff mode"));

    app.status_message = None;
    app.handle_key(KeyCode::Char('a'));
    assert_eq!(app.status_message.as_deref(), Some("N/A in diff mode"));

    app.status_message = None;
    app.handle_key(KeyCode::Char('1'));
    assert_eq!(app.status_message.as_deref(), Some("N/A in diff mode"));
}

#[test]
fn test_yank_command() {
    let mut app = make_categorical_app();
    app.handle_key(KeyCode::Char('y'));
    let msg = app.status_message.unwrap();
    assert!(msg.contains("before.csv"));
    assert!(msg.contains("after.csv"));
}

#[test]
fn test_yank_command_with_sort() {
    let mut app = make_categorical_app();
    app.sort_order = Some(SortOrder::Desc);
    app.handle_key(KeyCode::Char('y'));
    let msg = app.status_message.unwrap();
    assert!(msg.contains("--sort desc"));
}

#[test]
fn test_sorted_entries_desc() {
    let mut app = make_categorical_app();
    app.sort_order = Some(SortOrder::Desc);
    let entries = app.sorted_entries();
    // Tokyo: |50|, Osaka: |-20|=20, Nagoya: 0
    assert_eq!(entries[0].label, "Tokyo");
    assert_eq!(entries[1].label, "Osaka");
    assert_eq!(entries[2].label, "Nagoya");
}

#[test]
fn test_sorted_entries_asc() {
    let mut app = make_categorical_app();
    app.sort_order = Some(SortOrder::Asc);
    let entries = app.sorted_entries();
    assert_eq!(entries[0].label, "Nagoya");
    assert_eq!(entries[1].label, "Osaka");
    assert_eq!(entries[2].label, "Tokyo");
}

#[test]
fn test_table_rows_categorical() {
    let app = make_categorical_app();
    let rows = app.table_rows();
    assert_eq!(rows.len(), 3);
    // First row: Tokyo, 100.0, 150.0, +50.0, +50.0%, ▲
    assert_eq!(rows[0][0], "Tokyo");
    assert_eq!(rows[0][1], "100.0");
    assert_eq!(rows[0][2], "150.0");
    assert_eq!(rows[0][3], "+50.0");
    assert_eq!(rows[0][4], "+50.0%");
    assert_eq!(rows[0][5], "▲");
}

#[test]
fn test_table_rows_temporal() {
    let app = make_temporal_app();
    let rows = app.table_rows();
    assert_eq!(rows.len(), 3);
    // First row: 2024-01, 100.0, 120.0, +20.0, +20.0%, ▲
    assert_eq!(rows[0][0], "2024-01");
    assert_eq!(rows[0][1], "100.0");
    assert_eq!(rows[0][2], "120.0");
    assert_eq!(rows[0][3], "+20.0");
    assert_eq!(rows[0][4], "+20.0%");
    assert_eq!(rows[0][5], "▲");
}

#[test]
fn test_table_rows_decrease_marker() {
    let app = make_categorical_app();
    let rows = app.table_rows();
    // Osaka row: delta = -20
    assert_eq!(rows[1][5], "▼");
}

#[test]
fn test_table_rows_unchanged_marker() {
    let app = make_categorical_app();
    let rows = app.table_rows();
    // Nagoya row: delta = 0
    assert_eq!(rows[2][5], "─");
}

#[test]
fn test_overall_pct() {
    let app = make_categorical_app();
    assert!(app.overall_pct().is_some());
    let pct = app.overall_pct().unwrap();
    assert!((pct - 7.9).abs() < 0.01);
}

#[test]
fn test_temporal_overall_pct() {
    let app = make_temporal_app();
    let pct = app.overall_pct().unwrap();
    assert!((pct - 22.2).abs() < 0.01);
}
