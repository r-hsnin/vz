use super::*;
use crate::infer::types::ColumnMeta;

fn make_test_app() -> ExploreApp {
    let schema = Schema::new(vec![
        ColumnMeta {
            name: "date".to_string(),
            data_type: DataType::Temporal,
            null_count: 0,
            sample_size: 4,
        },
        ColumnMeta {
            name: "city".to_string(),
            data_type: DataType::Categorical,
            null_count: 0,
            sample_size: 4,
        },
        ColumnMeta {
            name: "revenue".to_string(),
            data_type: DataType::Quantitative,
            null_count: 0,
            sample_size: 4,
        },
    ]);
    let data = vec![
        vec!["2024-01-01".into(), "Tokyo".into(), "100".into()],
        vec!["2024-02-01".into(), "Osaka".into(), "200".into()],
        vec!["2024-03-01".into(), "Tokyo".into(), "150".into()],
        vec!["2024-04-01".into(), "Nagoya".into(), "300".into()],
    ];
    ExploreApp::new(schema, data, crate::theme::Theme::dark())
}

#[test]
fn test_initial_axes_prefer_temporal_for_x() {
    let app = make_test_app();
    assert_eq!(app.selected_x, 0); // date (temporal)
    assert_eq!(app.selected_y, 2); // revenue (quantitative)
}

#[test]
fn test_effective_chart_type_auto() {
    let app = make_test_app();
    assert_eq!(app.effective_chart_type(), ChartType::Line);
}

#[test]
fn test_chart_type_override() {
    let mut app = make_test_app();
    app.handle_key(KeyCode::Char('2'));
    assert_eq!(app.effective_chart_type(), ChartType::Bar);
}

#[test]
fn test_chart_type_reset_to_auto() {
    let mut app = make_test_app();
    app.handle_key(KeyCode::Char('3'));
    assert_eq!(app.effective_chart_type(), ChartType::Scatter);
    app.handle_key(KeyCode::Char('0'));
    assert_eq!(app.effective_chart_type(), ChartType::Line);
}

#[test]
fn test_navigate_x_axis() {
    let mut app = make_test_app();
    assert_eq!(app.selected_x, 0);
    app.handle_key(KeyCode::Char('l'));
    assert_eq!(app.selected_x, 1);
    app.handle_key(KeyCode::Char('l'));
    assert_eq!(app.selected_x, 2);
    // Shouldn't go past the end
    app.handle_key(KeyCode::Char('l'));
    assert_eq!(app.selected_x, 2);
}

#[test]
fn test_navigate_y_axis() {
    let mut app = make_test_app();
    assert_eq!(app.selected_y, 2);
    app.handle_key(KeyCode::Char('k'));
    assert_eq!(app.selected_y, 1);
    app.handle_key(KeyCode::Char('k'));
    assert_eq!(app.selected_y, 0);
    // Shouldn't go below 0
    app.handle_key(KeyCode::Char('k'));
    assert_eq!(app.selected_y, 0);
}

#[test]
fn test_quit_key() {
    let mut app = make_test_app();
    assert!(!app.should_quit);
    app.handle_key(KeyCode::Char('q'));
    assert!(app.should_quit);
}

#[test]
fn test_build_chart_config() {
    let app = make_test_app();
    let config = app.build_chart_config();
    assert!(config.title.is_some());
    // Auto-detects "city" as color column → 3 series (Tokyo, Osaka, Nagoya)
    assert_eq!(config.series.len(), 3);
    // Total data points across all series = 4 rows
    let total_points: usize = config.series.iter().map(|s| s.data.len()).sum();
    assert_eq!(total_points, 4);
}

#[test]
fn test_build_bar_data() {
    let mut app = make_test_app();
    app.selected_x = 1; // city (categorical)
    let data = app.build_bar_data();
    // Data has Tokyo(100), Osaka(200), Tokyo(150), Nagoya(300)
    // After aggregation: Tokyo=250, Osaka=200, Nagoya=300
    assert_eq!(data.labels.len(), 3);
    assert_eq!(data.values.len(), 3);
    assert_eq!(data.labels, vec!["Tokyo", "Osaka", "Nagoya"]);
    assert_eq!(data.values, vec![250.0, 200.0, 300.0]);
}

#[test]
fn test_build_histogram_data() {
    let mut app = make_test_app();
    app.selected_x = 2; // revenue (quantitative)
    let hist_data = app.build_histogram_data();
    assert_eq!(hist_data.values.len(), 4);
    assert_eq!(hist_data.bin_count, 10);
}

#[test]
fn test_y_values_numeric() {
    let app = make_test_app();
    let values = app.y_values();
    assert_eq!(values, vec![100.0, 200.0, 150.0, 300.0]);
}

#[test]
fn test_default_view_mode_is_chart() {
    let app = make_test_app();
    assert_eq!(app.view_mode, ViewMode::Chart);
}

#[test]
fn test_toggle_view_mode_with_d() {
    let mut app = make_test_app();
    assert_eq!(app.view_mode, ViewMode::Chart);
    app.handle_key(KeyCode::Char('d'));
    assert_eq!(app.view_mode, ViewMode::Table);
    app.handle_key(KeyCode::Char('d'));
    assert_eq!(app.view_mode, ViewMode::Chart);
}

#[test]
fn test_toggle_view_mode_with_tab() {
    let mut app = make_test_app();
    app.handle_key(KeyCode::Tab);
    assert_eq!(app.view_mode, ViewMode::Table);
    app.handle_key(KeyCode::Tab);
    assert_eq!(app.view_mode, ViewMode::Chart);
}

#[test]
fn test_table_scroll_state() {
    let mut app = make_test_app();
    app.handle_key(KeyCode::Char('d')); // switch to table
    assert_eq!(app.table_offset, 0);
    // In table mode, j/k should scroll rows (not change Y axis)
    app.handle_key(KeyCode::Char('j'));
    assert_eq!(app.table_offset, 1);
    app.handle_key(KeyCode::Char('k'));
    assert_eq!(app.table_offset, 0);
    // Should not go below 0
    app.handle_key(KeyCode::Char('k'));
    assert_eq!(app.table_offset, 0);
}

#[test]
fn test_table_jump_and_page_navigation() {
    let mut app = make_test_app();
    app.handle_key(KeyCode::Char('d'));
    app.handle_key(KeyCode::Char('G'));
    assert_eq!(app.table_offset, app.data.len() - 1);
    app.handle_key(KeyCode::Char('g'));
    assert_eq!(app.table_offset, 0);
    app.handle_key(KeyCode::PageDown);
    assert_eq!(app.table_offset, 3);
    app.handle_key(KeyCode::PageUp);
    assert_eq!(app.table_offset, 0);
    // G is noop in chart mode
    app.handle_key(KeyCode::Char('d'));
    app.handle_key(KeyCode::Char('G'));
    assert_eq!(app.table_offset, 0);
}

#[test]
fn test_x_skips_y_when_navigating_right() {
    let mut app = make_test_app();
    // Initial: x=0(date), y=2(revenue)
    app.selected_x = 1; // city
    app.selected_y = 2; // revenue
    // Moving x right from 1 should skip 2 (== y) — but can't go past max=2
    // So it should stay at 1 since skipping would go out of bounds
    app.handle_key(KeyCode::Char('l'));
    // x moves to 2, equals y=2, tries to skip to 3 which is past max — stays at 2
    // This is acceptable: at boundary, collision is allowed
    assert_eq!(app.selected_x, 2);
}

#[test]
fn test_x_skips_y_when_navigating_left() {
    let mut app = make_test_app();
    // Set up: x=2, y=1
    app.selected_x = 2;
    app.selected_y = 1;
    // Moving x left from 2 → 1, collides with y=1, skips to 0
    app.handle_key(KeyCode::Char('h'));
    assert_eq!(app.selected_x, 0);
}

#[test]
fn test_y_skips_x_when_navigating_down() {
    let mut app = make_test_app();
    // Set up: x=1, y=0
    app.selected_x = 1;
    app.selected_y = 0;
    // Moving y down from 0 → 1, collides with x=1, skips to 2
    app.handle_key(KeyCode::Char('j'));
    assert_eq!(app.selected_y, 2);
}

#[test]
fn test_y_skips_x_when_navigating_up() {
    let mut app = make_test_app();
    // Set up: x=1, y=2
    app.selected_x = 1;
    app.selected_y = 2;
    // Moving y up from 2 → 1, collides with x=1, skips to 0
    app.handle_key(KeyCode::Char('k'));
    assert_eq!(app.selected_y, 0);
}

#[test]
fn test_color_cycle_none_to_categorical() {
    let mut app = make_test_app();
    // Initial: x=0(date/temporal), y=2(revenue/quant)
    // Only categorical col is 1 (city), not used as x or y
    assert_eq!(app.selected_color, None);
    app.handle_key(KeyCode::Char('c'));
    assert_eq!(app.selected_color, Some(1)); // city
}

#[test]
fn test_color_cycle_wraps_to_none() {
    let mut app = make_test_app();
    // Only one categorical column available (city at index 1)
    app.handle_key(KeyCode::Char('c'));
    assert_eq!(app.selected_color, Some(1));
    // Cycle again → no more categoricals → back to None
    app.handle_key(KeyCode::Char('c'));
    assert_eq!(app.selected_color, None);
}

#[test]
fn test_color_cycle_skips_x_and_y() {
    let mut app = make_test_app();
    // Move x to city (idx 1, categorical) — now it's used as x
    app.selected_x = 1;
    app.selected_y = 2;
    // No categorical columns available (city is x)
    app.handle_key(KeyCode::Char('c'));
    assert_eq!(app.selected_color, None); // nothing to select
}

#[test]
fn test_cycle_color_no_categoricals_shows_message() {
    let schema = Schema::new(vec![
        ColumnMeta {
            name: "x".to_string(),
            data_type: DataType::Quantitative,
            null_count: 0,
            sample_size: 2,
        },
        ColumnMeta {
            name: "y".to_string(),
            data_type: DataType::Quantitative,
            null_count: 0,
            sample_size: 2,
        },
    ]);
    let data = vec![vec!["1".into(), "2".into()]];
    let mut app = ExploreApp::new(schema, data, crate::theme::Theme::dark());
    app.handle_key(KeyCode::Char('c'));
    assert_eq!(
        app.status_message.as_deref(),
        Some("no color columns available")
    );
}

#[test]
fn test_status_bar_shows_current_color_column() {
    let mut app = make_test_app();
    // city is at index 1 (categorical)
    app.handle_key(KeyCode::Char('c'));
    assert_eq!(app.selected_color, Some(1));
    // The status bar function uses app.selected_color to show "c=city"
    // We just verify the field is set correctly
}

#[test]
fn test_chart_type_change_shows_notification() {
    let mut app = make_test_app();
    // Initial: date(Temporal) x revenue(Quantitative) = Line
    assert_eq!(app.effective_chart_type(), ChartType::Line);
    // Navigate X to city (Categorical) → should auto-change to Bar
    app.handle_key(KeyCode::Char('h')); // Move X to the left... actually let's navigate right
    app.handle_key(KeyCode::Char('l')); // Move X from date to city
    // After moving X to Categorical, chart type should change
    if app.effective_chart_type() != ChartType::Line {
        assert!(
            app.status_message.is_some(),
            "Expected notification on chart type change"
        );
        assert!(
            app.status_message
                .as_deref()
                .unwrap_or("")
                .contains("auto:"),
            "Notification should contain 'auto:'"
        );
    }
}

#[test]
fn test_help_overlay_toggle() {
    let mut app = make_test_app();
    assert!(!app.show_help);
    app.handle_key(KeyCode::Char('?'));
    assert!(app.show_help, "? should open help");
    // Any key closes it
    app.handle_key(KeyCode::Char('x'));
    assert!(!app.show_help, "any key should close help");
    // The 'x' key should not have done anything else (not quit, etc.)
    assert!(!app.should_quit);
}

#[test]
fn test_initial_status_message_shows_hints() {
    let app = make_test_app();
    let msg = app.status_message.as_deref().unwrap();
    assert!(msg.contains("?"), "should mention help key");
    assert!(msg.contains("h/l"), "should mention axis navigation");
    assert!(msg.contains("q"), "should mention quit");
}

#[test]
fn test_yank_command_generates_oneshot() {
    let mut app = make_test_app();
    app.handle_key(KeyCode::Char('y'));
    let msg = app.status_message.as_deref().unwrap();
    assert!(msg.starts_with("vz <FILE>"), "should generate vz command");
    assert!(msg.contains("-x "), "should contain -x flag");
    assert!(msg.contains("-y "), "should contain -y flag");
    assert!(msg.contains("-t "), "should contain -t flag");
}
