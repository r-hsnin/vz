use super::*;

#[test]
fn test_color_legend_hint_basic() {
    let headers = vec!["city".to_string(), "revenue".to_string()];
    let rows = vec![
        vec!["Tokyo".to_string(), "1000".to_string()],
        vec!["Osaka".to_string(), "2000".to_string()],
    ];
    let result = color_legend_hint("city", &headers, &rows, crate::render::SERIES_COLORS);
    assert!(result.contains("Tokyo=cyan"));
    assert!(result.contains("Osaka=yellow"));
}

#[test]
fn test_color_legend_hint_missing_column() {
    let headers = vec!["x".to_string()];
    let rows = vec![vec!["a".to_string()]];
    let result = color_legend_hint("missing", &headers, &rows, crate::render::SERIES_COLORS);
    assert_eq!(result, "color=missing");
}

#[test]
fn test_unused_columns_hint_none_when_all_used() {
    let rec = ChartRecommendation {
        chart_type: ChartType::Line,
        x_column: "date".to_string(),
        y_column: Some("revenue".to_string()),
        color_column: Some("city".to_string()),
    };
    let headers = vec![
        "date".to_string(),
        "revenue".to_string(),
        "city".to_string(),
    ];
    assert_eq!(unused_columns_hint(&rec, &headers), None);
}

#[test]
fn test_unused_columns_hint_shows_unused() {
    let rec = ChartRecommendation {
        chart_type: ChartType::Line,
        x_column: "date".to_string(),
        y_column: Some("revenue".to_string()),
        color_column: None,
    };
    let headers = vec![
        "date".to_string(),
        "revenue".to_string(),
        "city".to_string(),
        "profit".to_string(),
    ];
    let hint = unused_columns_hint(&rec, &headers).unwrap();
    assert!(hint.contains("+2"));
    assert!(hint.contains("city"));
    assert!(hint.contains("profit"));
}

#[test]
fn test_unused_columns_hint_truncates_many() {
    let rec = ChartRecommendation {
        chart_type: ChartType::Line,
        x_column: "x".to_string(),
        y_column: None,
        color_column: None,
    };
    let headers = vec![
        "x".to_string(),
        "a".to_string(),
        "b".to_string(),
        "c".to_string(),
        "d".to_string(),
    ];
    let hint = unused_columns_hint(&rec, &headers).unwrap();
    assert!(hint.contains("+4"));
    assert!(hint.contains("a"));
    assert!(hint.contains("b"));
    assert!(hint.contains('…'));
}

#[test]
fn test_unused_columns_hint_single_suggests_command() {
    let rec = ChartRecommendation {
        chart_type: ChartType::Line,
        x_column: "date".to_string(),
        y_column: Some("revenue".to_string()),
        color_column: None,
    };
    let headers = vec![
        "date".to_string(),
        "revenue".to_string(),
        "profit".to_string(),
    ];
    let hint = unused_columns_hint(&rec, &headers).unwrap();
    assert!(hint.contains("+1: profit"));
    assert!(hint.contains("-y revenue,profit"));
}

#[test]
fn test_compute_y_stats_basic() {
    let rows = vec![
        vec!["2024-01".to_string(), "100".to_string()],
        vec!["2024-02".to_string(), "200".to_string()],
        vec!["2024-03".to_string(), "300".to_string()],
    ];
    let (min, max) = compute_y_stats(&rows, 1).unwrap();
    assert!((min - 100.0).abs() < f64::EPSILON);
    assert!((max - 300.0).abs() < f64::EPSILON);
}

#[test]
fn test_compute_y_stats_empty() {
    let rows: Vec<Vec<String>> = vec![];
    assert_eq!(compute_y_stats(&rows, 0), None);
}

#[test]
fn test_agg_label_display() {
    assert_eq!(agg_label(AggFunction::Sum), "sum");
    assert_eq!(agg_label(AggFunction::Mean), "mean");
    assert_eq!(agg_label(AggFunction::Count), "count");
    assert_eq!(agg_label(AggFunction::Max), "max");
    assert_eq!(agg_label(AggFunction::Min), "min");
}

#[test]
fn test_build_summary_parts_basic() {
let rec = ChartRecommendation {
    chart_type: ChartType::Line,
    x_column: "date".to_string(),
    y_column: Some("revenue".to_string()),
    color_column: None,
};
let headers = vec!["date".to_string(), "revenue".to_string()];
let rows = vec![
    vec!["2024-01".to_string(), "1000".to_string()],
    vec!["2024-02".to_string(), "2000".to_string()],
];
let parts = build_summary_parts(&SummaryContext {
    recommendation: &rec,
    chart_type: ChartType::Line,
    headers: &headers,
    rows: &rows,
    extra_y_columns: &[],
    agg: AggFunction::Sum,
    agg_stats: None,
    skipped_rows: 0,
    series_colors: crate::render::SERIES_COLORS,
});
assert_eq!(parts[0], "Line");
assert_eq!(parts[1], "x=date");
assert!(parts[2].starts_with("y=revenue"));
assert!(parts[2].contains("1.0k"));
assert!(parts[2].contains("2.0k"));
assert!(parts.iter().any(|p| p.contains("2 rows")));
}

#[test]
fn test_build_summary_parts_with_agg_stats() {
let rec = ChartRecommendation {
    chart_type: ChartType::Bar,
    x_column: "city".to_string(),
    y_column: Some("revenue".to_string()),
    color_column: None,
};
let headers = vec!["city".to_string(), "revenue".to_string()];
let rows = vec![
    vec!["Tokyo".to_string(), "1000".to_string()],
    vec!["Tokyo".to_string(), "2000".to_string()],
];
// With agg_stats override, summary should use provided stats
let parts = build_summary_parts(&SummaryContext {
    recommendation: &rec,
    chart_type: ChartType::Bar,
    headers: &headers,
    rows: &rows,
    extra_y_columns: &[],
    agg: AggFunction::Sum,
    agg_stats: Some((800.0, 4200.0)),
    skipped_rows: 0,
    series_colors: crate::render::SERIES_COLORS,
});
assert!(
    parts[2].contains("4.2k"),
    "Expected 4.2k in parts: {:?}",
    parts
);
}

#[test]
fn test_build_summary_parts_non_sum_agg() {
let rec = ChartRecommendation {
    chart_type: ChartType::Bar,
    x_column: "city".to_string(),
    y_column: Some("revenue".to_string()),
    color_column: None,
};
let headers = vec!["city".to_string(), "revenue".to_string()];
let rows = vec![vec!["Tokyo".to_string(), "1000".to_string()]];
let parts = build_summary_parts(&SummaryContext {
    recommendation: &rec,
    chart_type: ChartType::Bar,
    headers: &headers,
    rows: &rows,
    extra_y_columns: &[],
    agg: AggFunction::Mean,
    agg_stats: None,
    skipped_rows: 0,
    series_colors: crate::render::SERIES_COLORS,
});
// Should show mean(revenue) without range (since range is misleading for non-sum)
assert!(
    parts[2].contains("mean(revenue)"),
    "Expected mean(revenue): {:?}",
    parts
);
assert!(
    !parts[2].contains('–'),
    "Should not contain range for non-sum agg"
);
}

#[test]
fn test_build_summary_parts_extra_y() {
let rec = ChartRecommendation {
    chart_type: ChartType::Line,
    x_column: "date".to_string(),
    y_column: Some("revenue".to_string()),
    color_column: None,
};
let headers = vec![
    "date".to_string(),
    "revenue".to_string(),
    "profit".to_string(),
];
let rows = vec![vec![
    "2024-01".to_string(),
    "1000".to_string(),
    "200".to_string(),
]];
let extra = vec![("profit".to_string(), None)];
let parts = build_summary_parts(&SummaryContext {
    recommendation: &rec,
    chart_type: ChartType::Line,
    headers: &headers,
    rows: &rows,
    extra_y_columns: &extra,
    agg: AggFunction::Sum,
    agg_stats: None,
    skipped_rows: 0,
    series_colors: crate::render::SERIES_COLORS,
});
assert!(
    parts.iter().any(|p| p.contains("y+=profit")),
    "Expected y+=profit: {:?}",
    parts
);
}

#[test]
fn test_sparkline_basic() {
let rows = vec![
    vec!["a".to_string(), "1".to_string()],
    vec!["b".to_string(), "5".to_string()],
    vec!["c".to_string(), "3".to_string()],
    vec!["d".to_string(), "10".to_string()],
];
let spark = sparkline(&rows, 1).unwrap();
assert_eq!(spark.chars().count(), 4);
// First value (1) should be lowest block, last (10) should be highest
let chars: Vec<char> = spark.chars().collect();
assert_eq!(chars[0], '▁'); // min value
assert_eq!(chars[3], '█'); // max value
}

#[test]
fn test_sparkline_single_value_returns_none() {
let rows = vec![vec!["a".to_string(), "5".to_string()]];
assert!(sparkline(&rows, 1).is_none());
}

#[test]
fn test_sparkline_constant_values() {
let rows = vec![
    vec!["a".to_string(), "5".to_string()],
    vec!["b".to_string(), "5".to_string()],
    vec!["c".to_string(), "5".to_string()],
];
let spark = sparkline(&rows, 1).unwrap();
// All same value → all middle blocks
assert!(spark.chars().all(|c| c == '▄'));
}

#[test]
fn test_trend_annotation_uptrend() {
let rows = vec![
    vec!["a".to_string(), "100".to_string()],
    vec!["b".to_string(), "200".to_string()],
];
let trend = trend_annotation(&rows, 1).unwrap();
assert!(
    trend.contains('↑'),
    "Expected ↑ for uptrend, got: {}",
    trend
);
assert!(trend.contains("+100%"), "Expected +100%, got: {}", trend);
}

#[test]
fn test_trend_annotation_downtrend() {
let rows = vec![
    vec!["a".to_string(), "100".to_string()],
    vec!["b".to_string(), "50".to_string()],
];
let trend = trend_annotation(&rows, 1).unwrap();
assert!(
    trend.contains('↓'),
    "Expected ↓ for downtrend, got: {}",
    trend
);
}

#[test]
fn test_trend_annotation_stable() {
let rows = vec![
    vec!["a".to_string(), "100".to_string()],
    vec!["b".to_string(), "103".to_string()],
];
let trend = trend_annotation(&rows, 1).unwrap();
assert!(trend.contains('→'), "Expected → for stable, got: {}", trend);
assert!(
    trend.contains("stable"),
    "Expected 'stable', got: {}",
    trend
);
}

#[test]
fn test_trend_annotation_single_row_returns_none() {
let rows = vec![vec!["a".to_string(), "100".to_string()]];
assert!(trend_annotation(&rows, 1).is_none());
}

#[test]
fn test_truncate_to_width_short_string() {
assert_eq!(truncate_to_width("hello", 80), "hello");
}

#[test]
fn test_truncate_to_width_exact() {
assert_eq!(truncate_to_width("12345", 5), "12345");
}

#[test]
fn test_truncate_to_width_overflow() {
let result = truncate_to_width("abcdefghij", 6);
assert_eq!(result, "abcde…");
assert_eq!(result.chars().count(), 6);
}

#[test]
fn test_truncate_to_width_one() {
assert_eq!(truncate_to_width("hello", 1), "…");
}

#[test]
fn test_build_summary_parts_shows_skipped_rows() {
let rec = ChartRecommendation {
    chart_type: ChartType::Line,
    x_column: "date".to_string(),
    y_column: Some("revenue".to_string()),
    color_column: None,
};
let headers = vec!["date".to_string(), "revenue".to_string()];
let rows = vec![
    vec!["2024-01".to_string(), "1000".to_string()],
    vec!["2024-02".to_string(), "N/A".to_string()],
    vec!["2024-03".to_string(), "2000".to_string()],
];
let parts = build_summary_parts(&SummaryContext {
    recommendation: &rec,
    chart_type: ChartType::Line,
    headers: &headers,
    rows: &rows,
    extra_y_columns: &[],
    agg: AggFunction::Sum,
    agg_stats: None,
    skipped_rows: 1,
    series_colors: crate::render::SERIES_COLORS,
});
assert!(
    parts.iter().any(|p| p.contains("3 rows (1 skipped)")),
    "Expected '3 rows (1 skipped)' in parts: {:?}",
    parts
);
}

#[test]
fn test_truncate_to_width_fits() {
assert_eq!(truncate_to_width("hello", 10), "hello");
}

#[test]
fn test_truncate_to_width_truncates() {
let result = truncate_to_width("hello world", 6);
assert_eq!(result, "hello…");
}

#[test]
fn test_truncate_to_width_min() {
assert_eq!(truncate_to_width("abc", 1), "…");
}
