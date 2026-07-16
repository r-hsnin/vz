use super::super::*;

#[test]
fn test_resolve_chart_type_default() {
    let rec = ChartRecommendation {
        chart_type: ChartType::Line,
        x_column: "date".to_string(),
        y_column: Some("revenue".to_string()),
        color_column: None,
    };
    assert_eq!(resolve_chart_type(&rec, None), ChartType::Line);
}

#[test]
fn test_resolve_chart_type_override() {
    let rec = ChartRecommendation {
        chart_type: ChartType::Line,
        x_column: "date".to_string(),
        y_column: Some("revenue".to_string()),
        color_column: None,
    };
    assert_eq!(
        resolve_chart_type(&rec, Some(crate::cli::ChartTypeArg::Bar)),
        ChartType::Bar
    );
    assert_eq!(
        resolve_chart_type(&rec, Some(crate::cli::ChartTypeArg::Scatter)),
        ChartType::Scatter
    );
    assert_eq!(
        resolve_chart_type(&rec, Some(crate::cli::ChartTypeArg::Histogram)),
        ChartType::Histogram
    );
}

#[test]
fn test_resolve_chart_type_invalid_falls_back() {
    let rec = ChartRecommendation {
        chart_type: ChartType::Line,
        x_column: "date".to_string(),
        y_column: Some("revenue".to_string()),
        color_column: None,
    };
    // With ValueEnum, invalid types are rejected at parse time by clap.
    // None falls back to recommendation.
    assert_eq!(resolve_chart_type(&rec, None), ChartType::Line);
}

#[test]
fn test_column_index_found() {
    let headers = vec![
        "date".to_string(),
        "city".to_string(),
        "revenue".to_string(),
    ];
    assert_eq!(data_builder::column_index(&headers, "city"), Some(1));
    assert_eq!(data_builder::column_index(&headers, "revenue"), Some(2));
}

#[test]
fn test_column_index_not_found() {
    let headers = vec!["date".to_string(), "city".to_string()];
    assert_eq!(data_builder::column_index(&headers, "nonexistent"), None);
}

#[test]
fn test_build_chart_config_line() {
    let rec = ChartRecommendation {
        chart_type: ChartType::Line,
        x_column: "date".to_string(),
        y_column: Some("revenue".to_string()),
        color_column: None,
    };
    let headers = vec![
        "date".to_string(),
        "city".to_string(),
        "revenue".to_string(),
    ];
    let rows = vec![
        vec![
            "2024-01-01".to_string(),
            "Tokyo".to_string(),
            "1000".to_string(),
        ],
        vec![
            "2024-02-01".to_string(),
            "Osaka".to_string(),
            "1500".to_string(),
        ],
        vec![
            "2024-03-01".to_string(),
            "Tokyo".to_string(),
            "1200".to_string(),
        ],
    ];

    let config = build_chart_config(&rec, &headers, &rows);
    assert_eq!(config.series.len(), 1);
    assert_eq!(config.series[0].data.len(), 3);
    assert!(config.title.unwrap().contains("revenue"));
}

#[test]
fn test_build_bar_data() {
    let rec = ChartRecommendation {
        chart_type: ChartType::Bar,
        x_column: "city".to_string(),
        y_column: Some("revenue".to_string()),
        color_column: None,
    };
    let headers = vec!["city".to_string(), "revenue".to_string()];
    let rows = vec![
        vec!["Tokyo".to_string(), "1000".to_string()],
        vec!["Osaka".to_string(), "1500".to_string()],
    ];

    let (data, rows_used) = build_bar_data(&rec, &headers, &rows, AggFunction::Sum);
    assert_eq!(data.labels, vec!["Tokyo", "Osaka"]);
    assert_eq!(data.values, vec![1000.0, 1500.0]);
    assert_eq!(rows_used, 2);
}

#[test]
fn test_build_bar_data_aggregates_duplicates() {
    let rec = ChartRecommendation {
        chart_type: ChartType::Bar,
        x_column: "city".to_string(),
        y_column: Some("revenue".to_string()),
        color_column: None,
    };
    let headers = vec!["city".to_string(), "revenue".to_string()];
    let rows = vec![
        vec!["Tokyo".to_string(), "1000".to_string()],
        vec!["Osaka".to_string(), "1500".to_string()],
        vec!["Tokyo".to_string(), "2000".to_string()],
        vec!["Osaka".to_string(), "500".to_string()],
    ];

    let (data, rows_used) = build_bar_data(&rec, &headers, &rows, AggFunction::Sum);
    assert_eq!(data.labels, vec!["Tokyo", "Osaka"]);
    assert_eq!(data.values, vec![3000.0, 2000.0]); // Summed
    assert_eq!(rows_used, 4);
}

#[test]
fn test_generate_x_labels_simple() {
    let values: Vec<String> = vec![
        "2024-01-01".to_string(),
        "2024-02-01".to_string(),
        "2024-03-01".to_string(),
    ];
    let labels = data_builder::pick_evenly(&values, 5);
    // When values.len() < count, return all
    assert_eq!(labels, values);
}

#[test]
fn test_generate_x_labels_picks_evenly() {
    let values: Vec<String> = (0..10).map(|i| format!("2024-{:02}-01", i + 1)).collect();
    let labels = data_builder::pick_evenly(&values, 3);
    assert_eq!(labels.len(), 3);
    assert_eq!(labels[0], "2024-01-01"); // first
    assert_eq!(labels[2], "2024-10-01"); // last
}

#[test]
fn test_build_histogram_data() {
    let rec = ChartRecommendation {
        chart_type: ChartType::Histogram,
        x_column: "score".to_string(),
        y_column: None,
        color_column: None,
    };
    let headers = vec!["score".to_string()];
    let rows = vec![
        vec!["85".to_string()],
        vec!["90".to_string()],
        vec!["78".to_string()],
        vec!["92".to_string()],
    ];

    let data = build_histogram_data(&rec, &headers, &rows);
    assert_eq!(data.values.len(), 4);
    assert_eq!(data.bin_count, 10);
    assert_eq!(data.x_label, "score");
}

#[test]
fn test_render_oneshot_line_chart_produces_output() {
    let rec = ChartRecommendation {
        chart_type: ChartType::Line,
        x_column: "date".to_string(),
        y_column: Some("revenue".to_string()),
        color_column: None,
    };
    let headers = vec![
        "date".to_string(),
        "city".to_string(),
        "revenue".to_string(),
    ];
    let rows = vec![
        vec![
            "2024-01-01".to_string(),
            "Tokyo".to_string(),
            "1000".to_string(),
        ],
        vec![
            "2024-02-01".to_string(),
            "Osaka".to_string(),
            "1500".to_string(),
        ],
        vec![
            "2024-03-01".to_string(),
            "Tokyo".to_string(),
            "1200".to_string(),
        ],
        vec![
            "2024-04-01".to_string(),
            "Nagoya".to_string(),
            "800".to_string(),
        ],
        vec![
            "2024-05-01".to_string(),
            "Tokyo".to_string(),
            "2000".to_string(),
        ],
        vec![
            "2024-06-01".to_string(),
            "Osaka".to_string(),
            "1800".to_string(),
        ],
    ];

    // Build the chart config and render to buffer to verify output is non-trivial
    let config = build_chart_config(&rec, &headers, &rows);
    let area = Rect::new(0, 0, 80, 20);
    let mut buf = Buffer::empty(area);
    crate::render::render_chart_data(&crate::render::ChartData::Line(config), area, &mut buf);

    let mut output = Vec::new();
    print_buffer(&buf, &mut output).unwrap();
    let text = String::from_utf8(output).unwrap();

    // Should have multiple lines of output
    assert!(text.lines().count() >= 10);
    // Should contain chart border characters or braille
    assert!(text.contains('│') || text.contains('─') || text.contains('┌') || text.contains('⠁'));
}

#[test]
fn test_render_oneshot_bar_chart_produces_output() {
    let rec = ChartRecommendation {
        chart_type: ChartType::Bar,
        x_column: "city".to_string(),
        y_column: Some("revenue".to_string()),
        color_column: None,
    };
    let headers = vec!["city".to_string(), "revenue".to_string()];
    let rows = vec![
        vec!["Tokyo".to_string(), "1000".to_string()],
        vec!["Osaka".to_string(), "1500".to_string()],
        vec!["Nagoya".to_string(), "800".to_string()],
    ];

    let (data, _) = build_bar_data(&rec, &headers, &rows, AggFunction::Sum);
    let area = Rect::new(0, 0, 80, 20);
    let mut buf = Buffer::empty(area);
    crate::render::render_chart_data(&crate::render::ChartData::Bar(data), area, &mut buf);

    let mut output = Vec::new();
    print_buffer(&buf, &mut output).unwrap();
    let text = String::from_utf8(output).unwrap();

    assert!(text.lines().count() >= 10);
    // Bar chart should show bar values
    assert!(text.contains("1000") || text.contains("1500") || text.contains("800"));
}
