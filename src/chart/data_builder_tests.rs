use super::*;
use crate::cli::AggFunction;

#[test]
fn test_pick_evenly_small() {
    let items: Vec<String> = vec!["a", "b", "c"].into_iter().map(String::from).collect();
    assert_eq!(pick_evenly(&items, 5), items);
}

#[test]
fn test_pick_evenly_large() {
    let items: Vec<String> = (0..20).map(|i| format!("item_{}", i)).collect();
    let result = pick_evenly(&items, 5);
    assert_eq!(result.len(), 5);
    assert_eq!(result[0], "item_0");
    assert_eq!(result[4], "item_19");
}

#[test]
fn test_pick_evenly_empty() {
    let items: Vec<String> = vec![];
    assert_eq!(pick_evenly(&items, 5), Vec::<String>::new());
}

#[test]
fn test_is_non_numeric() {
    assert!(is_non_numeric(&["2024-01".into(), "2024-02".into()]));
    assert!(!is_non_numeric(&["1.0".into(), "2.5".into(), "3".into()]));
    assert!(!is_non_numeric(&[]));
}

#[test]
fn test_unique_ordered() {
    let vals: Vec<String> = vec!["a", "b", "a", "c", "b"]
        .into_iter()
        .map(String::from)
        .collect();
    assert_eq!(unique_ordered(&vals), vec!["a", "b", "c"]);
}

#[test]
fn test_aggregate_bar() {
    let rows = vec![
        vec!["Tokyo".into(), "1000".into()],
        vec!["Osaka".into(), "500".into()],
        vec!["Tokyo".into(), "2000".into()],
    ];
    let (data, used) = aggregate_bar(
        &rows,
        0,
        1,
        Some("Test".into()),
        "revenue".into(),
        AggFunction::Sum,
    );
    assert_eq!(data.labels, vec!["Tokyo", "Osaka"]);
    assert_eq!(data.values, vec![3000.0, 500.0]);
    assert_eq!(used, 3);
}

#[test]
fn test_aggregate_bar_with_non_parseable() {
    let rows = vec![
        vec!["Tokyo".into(), "1000".into()],
        vec!["Osaka".into(), "bad".into()],
        vec!["Tokyo".into(), "500".into()],
    ];
    let (data, used) = aggregate_bar(&rows, 0, 1, None, "y".into(), AggFunction::Sum);
    assert_eq!(data.labels, vec!["Tokyo"]);
    assert_eq!(data.values, vec![1500.0]);
    assert_eq!(used, 2);
}

#[test]
fn test_aggregate_bar_mean() {
    let rows = vec![
        vec!["Tokyo".into(), "1000".into()],
        vec!["Osaka".into(), "500".into()],
        vec!["Tokyo".into(), "3000".into()],
    ];
    let (data, used) = aggregate_bar(&rows, 0, 1, None, "y".into(), AggFunction::Mean);
    assert_eq!(data.labels, vec!["Tokyo", "Osaka"]);
    assert_eq!(data.values, vec![2000.0, 500.0]); // mean(1000,3000)=2000, mean(500)=500
    assert_eq!(used, 3);
}

#[test]
fn test_aggregate_bar_count() {
    let rows = vec![
        vec!["Tokyo".into(), "1000".into()],
        vec!["Osaka".into(), "500".into()],
        vec!["Tokyo".into(), "3000".into()],
        vec!["Tokyo".into(), "bad".into()], // count still counts this
    ];
    let (data, used) = aggregate_bar(&rows, 0, 1, None, "y".into(), AggFunction::Count);
    assert_eq!(data.labels, vec!["Tokyo", "Osaka"]);
    assert_eq!(data.values, vec![3.0, 1.0]); // count ignores Y parsability
    assert_eq!(used, 4);
}

#[test]
fn test_aggregate_bar_max_min() {
    let rows = vec![
        vec!["A".into(), "10".into()],
        vec!["A".into(), "30".into()],
        vec!["A".into(), "20".into()],
        vec!["B".into(), "5".into()],
    ];
    let (data_max, _) = aggregate_bar(&rows, 0, 1, None, "y".into(), AggFunction::Max);
    assert_eq!(data_max.values, vec![30.0, 5.0]);

    let (data_min, _) = aggregate_bar(&rows, 0, 1, None, "y".into(), AggFunction::Min);
    assert_eq!(data_min.values, vec![10.0, 5.0]);
}

#[test]
fn test_build_single_series() {
    let rows = vec![
        vec!["1.0".into(), "10.0".into()],
        vec!["2.0".into(), "20.0".into()],
        vec!["3.0".into(), "30.0".into()],
    ];
    let series = build_single_series(&rows, 0, 1, false, "Y".into());
    assert_eq!(series.data.len(), 3);
    assert_eq!(series.data[0], (1.0, 10.0));
    assert_eq!(series.data[2], (3.0, 30.0));
}

#[test]
fn test_build_grouped_series() {
    let rows = vec![
        vec!["2024-01".into(), "100".into(), "A".into()],
        vec!["2024-02".into(), "200".into(), "B".into()],
        vec!["2024-01".into(), "150".into(), "B".into()],
    ];
    let series = build_grouped_series(&rows, 0, 1, 2, true);
    assert_eq!(series.len(), 2);
    assert_eq!(series[0].name, "A");
    assert_eq!(series[1].name, "B");
    assert_eq!(series[0].data.len(), 1);
    assert_eq!(series[1].data.len(), 2);
}

#[test]
fn test_build_chart_config_single_series() {
    let rows = vec![
        vec!["2024-01".into(), "100".into()],
        vec!["2024-02".into(), "200".into()],
    ];
    let config = build_chart_config(&rows, 0, 1, None, "date".into(), "value".into(), None);
    assert_eq!(config.series.len(), 1);
    assert_eq!(config.series[0].data.len(), 2);
    assert!(config.x_labels.is_some()); // non-numeric X
}

#[test]
fn test_build_chart_config_multi_series() {
    let rows = vec![
        vec!["2024-01".into(), "100".into(), "A".into()],
        vec!["2024-01".into(), "200".into(), "B".into()],
        vec!["2024-02".into(), "150".into(), "A".into()],
    ];
    let config = build_chart_config(&rows, 0, 1, Some(2), "date".into(), "value".into(), None);
    assert_eq!(config.series.len(), 2);
}

#[test]
fn test_column_index() {
    let headers: Vec<String> = vec!["a".into(), "b".into(), "c".into()];
    assert_eq!(column_index(&headers, "b"), Some(1));
    assert_eq!(column_index(&headers, "z"), None);
}

#[test]
fn test_sample_rows_under_threshold() {
    // Under threshold: no sampling
    let rows: Vec<Vec<String>> = (0..100)
        .map(|i| vec![format!("{}", i), format!("{}", i * 10)])
        .collect();
    let sampled = sample_rows(&rows, 5000);
    assert_eq!(sampled.len(), 100);
}

#[test]
fn test_sample_rows_over_threshold() {
    // Over threshold: sampled down
    let rows: Vec<Vec<String>> = (0..10000)
        .map(|i| vec![format!("{}", i), format!("{}", i * 10)])
        .collect();
    let sampled = sample_rows(&rows, 5000);
    assert_eq!(sampled.len(), 5000);
    // First and last rows should be preserved (systematic sampling)
    assert_eq!(sampled[0], rows[0]);
    assert_eq!(sampled[4999], rows[9999]);
}

#[test]
fn test_sample_rows_empty() {
    let rows: Vec<Vec<String>> = vec![];
    let sampled = sample_rows(&rows, 5000);
    assert!(sampled.is_empty());
}

#[test]
fn test_build_chart_config_samples_large_data() {
    // 10k rows should be sampled for chart rendering
    let rows: Vec<Vec<String>> = (0..10000)
        .map(|i| vec![format!("{}", i), format!("{}", i * 2)])
        .collect();
    let config = build_chart_config(&rows, 0, 1, None, "x".into(), "y".into(), None);
    // Should have at most MAX_CHART_POINTS data points
    let total_points: usize = config.series.iter().map(|s| s.data.len()).sum();
    assert!(total_points <= MAX_CHART_POINTS);
}

#[test]
fn test_build_heatmap_data_basic() {
    let rows = vec![
        vec!["A".to_string(), "X".to_string()],
        vec!["A".to_string(), "Y".to_string()],
        vec!["B".to_string(), "X".to_string()],
        vec!["B".to_string(), "X".to_string()],
    ];
    let data = build_heatmap_data(&rows, 0, 1, Some("Test".to_string()));
    assert_eq!(data.row_labels, vec!["A", "B"]);
    assert_eq!(data.col_labels, vec!["X", "Y"]);
    assert_eq!(data.counts, vec![vec![1, 1], vec![2, 0]]);
    assert_eq!(data.max_count, 2);
    assert_eq!(data.title, Some("Test".to_string()));
}

#[test]
fn test_build_heatmap_data_empty() {
    let rows: Vec<Vec<String>> = vec![];
    let data = build_heatmap_data(&rows, 0, 1, None);
    assert!(data.row_labels.is_empty());
    assert!(data.col_labels.is_empty());
    assert!(data.counts.is_empty());
    assert_eq!(data.max_count, 0);
}

#[test]
fn test_build_heatmap_data_single_cell() {
    let rows = vec![
        vec!["A".to_string(), "X".to_string()],
        vec!["A".to_string(), "X".to_string()],
        vec!["A".to_string(), "X".to_string()],
    ];
    let data = build_heatmap_data(&rows, 0, 1, None);
    assert_eq!(data.row_labels, vec!["A"]);
    assert_eq!(data.col_labels, vec!["X"]);
    assert_eq!(data.counts, vec![vec![3]]);
    assert_eq!(data.max_count, 3);
}

#[test]
fn test_resolved_axes_from_explicit() {
    let headers = vec!["city".into(), "revenue".into(), "region".into()];
    let axes = ResolvedAxes::from_explicit(Some("city"), Some("revenue"), Some("region"), &headers);
    assert_eq!(axes.x_idx, 0);
    assert_eq!(axes.y_idx, 1);
    assert_eq!(axes.color_idx, Some(2));
    assert_eq!(axes.x_label, "city");
    assert_eq!(axes.y_label, "revenue");
}

#[test]
fn test_resolved_axes_from_explicit_defaults() {
    let headers = vec!["date".into(), "value".into()];
    let axes = ResolvedAxes::from_explicit(None, None, None, &headers);
    assert_eq!(axes.x_idx, 0);
    assert_eq!(axes.y_idx, 1);
    assert_eq!(axes.color_idx, None);
    assert_eq!(axes.x_label, "date");
    assert_eq!(axes.y_label, "value");
}

#[test]
fn test_resolved_axes_from_recommendation() {
    let headers = vec!["month".into(), "sales".into(), "city".into()];
    let axes = ResolvedAxes::from_recommendation("month", Some("sales"), Some("city"), &headers);
    assert_eq!(axes.x_idx, 0);
    assert_eq!(axes.y_idx, 1);
    assert_eq!(axes.color_idx, Some(2));
}

#[test]
fn test_resolved_axes_single_column() {
    let headers = vec!["values".into()];
    let axes = ResolvedAxes::from_explicit(None, None, None, &headers);
    assert_eq!(axes.x_idx, 0);
    assert_eq!(axes.y_idx, 0); // min(1, len-1) = min(1, 0) = 0
}

#[test]
fn test_collect_groups_sum() {
    let rows = vec![
        vec!["A".to_string(), "10".to_string()],
        vec!["B".to_string(), "20".to_string()],
        vec!["A".to_string(), "30".to_string()],
    ];
    let (groups, used) = collect_groups(&rows, 0, 1, AggFunction::Sum);
    assert_eq!(used, 3);
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].0, "A");
    assert_eq!(groups[0].1, vec![10.0, 30.0]);
    assert_eq!(groups[1].0, "B");
    assert_eq!(groups[1].1, vec![20.0]);
}

#[test]
fn test_collect_groups_count() {
    let rows = vec![
        vec!["X".to_string(), "ignored".to_string()],
        vec!["Y".to_string(), "also_ignored".to_string()],
        vec!["X".to_string(), "nope".to_string()],
    ];
    let (groups, used) = collect_groups(&rows, 0, 1, AggFunction::Count);
    assert_eq!(used, 3);
    assert_eq!(groups[0].0, "X");
    assert_eq!(groups[0].1.len(), 2); // two entries for X
}
