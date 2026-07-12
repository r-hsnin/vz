use super::*;

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

#[test]
fn test_build_grouped_series_shares_x_coordinates() {
    // Simulates sales.csv: each city has data at different row positions
    // but should share the same X coordinate space based on unique X values
    let rows = vec![
        vec![
            "2024-01-01".to_string(),
            "1000".to_string(),
            "Tokyo".to_string(),
        ],
        vec![
            "2024-02-01".to_string(),
            "1500".to_string(),
            "Osaka".to_string(),
        ],
        vec![
            "2024-03-01".to_string(),
            "1200".to_string(),
            "Tokyo".to_string(),
        ],
        vec![
            "2024-04-01".to_string(),
            "800".to_string(),
            "Nagoya".to_string(),
        ],
        vec![
            "2024-05-01".to_string(),
            "2000".to_string(),
            "Tokyo".to_string(),
        ],
        vec![
            "2024-06-01".to_string(),
            "1800".to_string(),
            "Osaka".to_string(),
        ],
    ];

    let series = data_builder::build_grouped_series(&rows, 0, 1, 2, true);

    // Should have 3 groups
    assert_eq!(series.len(), 3);

    let tokyo = series.iter().find(|s| s.name == "Tokyo").unwrap();
    let osaka = series.iter().find(|s| s.name == "Osaka").unwrap();
    let nagoya = series.iter().find(|s| s.name == "Nagoya").unwrap();

    // Tokyo has dates at positions 0, 2, 4 in unique_x order
    assert_eq!(tokyo.data.len(), 3);
    assert_eq!(tokyo.data[0].0, 0.0); // 2024-01-01 → index 0
    assert_eq!(tokyo.data[1].0, 2.0); // 2024-03-01 → index 2
    assert_eq!(tokyo.data[2].0, 4.0); // 2024-05-01 → index 4

    // Osaka has dates at positions 1, 5 in unique_x order
    assert_eq!(osaka.data.len(), 2);
    assert_eq!(osaka.data[0].0, 1.0); // 2024-02-01 → index 1
    assert_eq!(osaka.data[1].0, 5.0); // 2024-06-01 → index 5

    // Nagoya at position 3
    assert_eq!(nagoya.data.len(), 1);
    assert_eq!(nagoya.data[0].0, 3.0); // 2024-04-01 → index 3
}

#[test]
fn test_build_grouped_series_numeric_x() {
    // When X is numeric, use actual numeric values
    let rows = vec![
        vec!["10".to_string(), "100".to_string(), "A".to_string()],
        vec!["20".to_string(), "200".to_string(), "B".to_string()],
        vec!["30".to_string(), "150".to_string(), "A".to_string()],
    ];

    let series = data_builder::build_grouped_series(&rows, 0, 1, 2, false);

    let group_a = series.iter().find(|s| s.name == "A").unwrap();
    assert_eq!(group_a.data[0].0, 10.0);
    assert_eq!(group_a.data[1].0, 30.0);
}

#[test]
fn test_fit_labels_narrow_width() {
    let labels: Vec<String> = vec![
        "2024-01-01".to_string(),
        "2024-02-01".to_string(),
        "2024-03-01".to_string(),
        "2024-04-01".to_string(),
        "2024-05-01".to_string(),
        "2024-06-01".to_string(),
    ];
    // Small datasets (≤10) always show all labels regardless of width
    let result = fit_labels_to_width(&labels, 28);
    assert_eq!(result.len(), 6);
    assert_eq!(result[0], "2024-01-01");
    assert_eq!(result[5], "2024-06-01");
}

#[test]
fn test_fit_labels_wide_width() {
    let labels: Vec<String> = vec![
        "Jan".to_string(),
        "Feb".to_string(),
        "Mar".to_string(),
        "Apr".to_string(),
        "May".to_string(),
    ];
    // At width 80, each 4-char label needs 4 chars, so max 20 — all fit
    let result = fit_labels_to_width(&labels, 80);
    assert_eq!(result.len(), 5);
}

#[test]
fn test_fit_labels_large_dataset_reduces() {
    let labels: Vec<String> = (1..=20).map(|i| format!("2024-{:02}-01", i)).collect();
    // 20 labels > 10, so width-based reduction kicks in
    let result = fit_labels_to_width(&labels, 40);
    assert!(
        result.len() < 20,
        "Expected labels to be reduced for large dataset"
    );
    assert!(result.len() >= 2, "Should keep at least 2 labels");
    assert_eq!(result[0], "2024-01-01");
    assert_eq!(*result.last().unwrap(), "2024-20-01");
}

#[test]
fn test_fit_labels_empty() {
    let result = fit_labels_to_width(&[], 80);
    assert!(result.is_empty());
}

#[test]
fn test_adaptive_height_bar_few_categories() {
    let rec = ChartRecommendation {
        chart_type: ChartType::Bar,
        x_column: "city".to_string(),
        y_column: Some("revenue".to_string()),
        color_column: None,
    };
    let headers = vec!["city".to_string(), "revenue".to_string()];
    let rows = vec![
        vec!["Tokyo".to_string(), "1000".to_string()],
        vec!["Osaka".to_string(), "500".to_string()],
    ];
    let height = adaptive_height(ChartType::Bar, &rec, &headers, &rows);
    assert_eq!(height, 10); // 2 * 4 + 2 = 10
}

#[test]
fn test_adaptive_height_bar_many_categories() {
    let rec = ChartRecommendation {
        chart_type: ChartType::Bar,
        x_column: "city".to_string(),
        y_column: Some("revenue".to_string()),
        color_column: None,
    };
    let headers = vec!["city".to_string(), "revenue".to_string()];
    let rows: Vec<Vec<String>> = (0..10)
        .map(|i| vec![format!("City{}", i), "100".to_string()])
        .collect();
    let height = adaptive_height(ChartType::Bar, &rec, &headers, &rows);
    assert_eq!(height, DEFAULT_HEIGHT); // > 5 categories, use default
}

#[test]
fn test_adaptive_height_non_bar() {
    let rec = ChartRecommendation {
        chart_type: ChartType::Line,
        x_column: "date".to_string(),
        y_column: Some("revenue".to_string()),
        color_column: None,
    };
    let headers = vec!["date".to_string(), "revenue".to_string()];
    // 1 row → adaptive: 1*3+6=9, clamped to min 12
    let rows = vec![vec!["2024-01".to_string(), "100".to_string()]];
    let height = adaptive_height(ChartType::Line, &rec, &headers, &rows);
    assert_eq!(height, 12);
}

#[test]
fn test_adaptive_height_line_small_dataset() {
    let rec = ChartRecommendation {
        chart_type: ChartType::Line,
        x_column: "date".to_string(),
        y_column: Some("value".to_string()),
        color_column: None,
    };
    let headers = vec!["date".to_string(), "value".to_string()];
    // 3 rows → height = 3*3+6 = 15
    let rows = vec![
        vec!["2024-01".into(), "10".into()],
        vec!["2024-02".into(), "20".into()],
        vec!["2024-03".into(), "30".into()],
    ];
    let height = adaptive_height(ChartType::Line, &rec, &headers, &rows);
    assert_eq!(height, 15);
}

#[test]
fn test_adaptive_height_scatter_small_dataset() {
    let rec = ChartRecommendation {
        chart_type: ChartType::Scatter,
        x_column: "x".to_string(),
        y_column: Some("y".to_string()),
        color_column: None,
    };
    let headers = vec!["x".to_string(), "y".to_string()];
    // 2 rows → height = 2*3+6 = 12
    let rows = vec![vec!["1".into(), "2".into()], vec!["3".into(), "4".into()]];
    let height = adaptive_height(ChartType::Scatter, &rec, &headers, &rows);
    assert_eq!(height, 12);
}

#[test]
fn test_adaptive_height_line_large_dataset_uses_default() {
    let rec = ChartRecommendation {
        chart_type: ChartType::Line,
        x_column: "date".to_string(),
        y_column: Some("value".to_string()),
        color_column: None,
    };
    let headers = vec!["date".to_string(), "value".to_string()];
    // 10 rows → exceeds threshold, uses DEFAULT_HEIGHT
    let rows: Vec<Vec<String>> = (0..10)
        .map(|i| vec![format!("2024-{:02}", i + 1), format!("{}", i * 10)])
        .collect();
    let height = adaptive_height(ChartType::Line, &rec, &headers, &rows);
    assert_eq!(height, DEFAULT_HEIGHT);
}

#[test]
fn test_build_histogram_data_custom_bins() {
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

    let data = build_histogram_data_with_bins(&rec, &headers, &rows, Some(5));
    assert_eq!(data.bin_count, 5);
}

#[test]
fn test_build_histogram_data_default_bins_when_none() {
    let rec = ChartRecommendation {
        chart_type: ChartType::Histogram,
        x_column: "score".to_string(),
        y_column: None,
        color_column: None,
    };
    let headers = vec!["score".to_string()];
    let rows = vec![vec!["85".to_string()], vec!["90".to_string()]];

    let data = build_histogram_data_with_bins(&rec, &headers, &rows, None);
    assert_eq!(data.bin_count, 10);
}
