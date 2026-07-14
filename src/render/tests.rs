use super::*;

#[test]
fn test_axis_from_data() {
    let values = vec![10.0, 20.0, 30.0];
    let axis = Axis::from_data("revenue", &values);
    assert_eq!(axis.label, "revenue");
    // Nice numbers: min should be ≤ data_min, max should be ≥ data_max
    assert!(axis.min <= 10.0, "min {} should be ≤ 10", axis.min);
    assert!(axis.max >= 30.0, "max {} should be ≥ 30", axis.max);
    // Should be round nice numbers
    assert!(
        axis.min == 10.0 || axis.min == 0.0 || axis.min == 5.0,
        "min {} should be a nice number",
        axis.min
    );
}

#[test]
fn test_axis_normalize() {
    let axis = Axis {
        label: "x".to_string(),
        min: 0.0,
        max: 100.0,
    };
    assert!((axis.normalize(50.0) - 0.5).abs() < f64::EPSILON);
    assert!((axis.normalize(0.0) - 0.0).abs() < f64::EPSILON);
    assert!((axis.normalize(100.0) - 1.0).abs() < f64::EPSILON);
}

#[test]
fn test_axis_normalize_same_min_max() {
    let axis = Axis {
        label: "x".to_string(),
        min: 5.0,
        max: 5.0,
    };
    assert!((axis.normalize(5.0) - 0.5).abs() < f64::EPSILON);
}

#[test]
fn test_compute_bins_basic() {
    let values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
    let bins = compute_bins(&values, 5);
    assert_eq!(bins.len(), 5);
    let total_count: usize = bins.iter().map(|b| b.2).sum();
    assert_eq!(total_count, 10);
}

#[test]
fn test_compute_bins_empty() {
    let bins = compute_bins(&[], 5);
    assert!(bins.is_empty());
}

#[test]
fn test_compute_bins_single_value() {
    let values = vec![5.0, 5.0, 5.0];
    let bins = compute_bins(&values, 5);
    assert_eq!(bins.len(), 1);
    assert_eq!(bins[0].2, 3);
}

#[test]
fn test_series_creation() {
    let series = Series {
        name: "Revenue".to_string(),
        data: vec![(0.0, 100.0), (1.0, 200.0)],
    };
    assert_eq!(series.name, "Revenue");
    assert_eq!(series.data.len(), 2);
}

#[test]
fn test_tick_labels_basic() {
    let axis = Axis {
        label: "x".to_string(),
        min: 0.0,
        max: 100.0,
    };
    let labels = axis.tick_labels(5);
    // Nice numbers may produce slightly different count but should include 0 and 100
    assert!(!labels.is_empty());
    assert_eq!(labels[0], "0");
    assert_eq!(*labels.last().unwrap(), "100");
}

#[test]
fn test_tick_labels_large_numbers() {
    let axis = Axis {
        label: "revenue".to_string(),
        min: 0.0,
        max: 2000.0,
    };
    let labels = axis.tick_labels(3);
    // Nice numbers: should produce round tick values
    assert!(!labels.is_empty());
    assert_eq!(labels[0], "0");
    // All labels should be round numbers (formatted as 0, 500, 1.0k, 1.5k, 2.0k, etc.)
    for label in &labels {
        assert!(
            label.ends_with('k') || label.parse::<f64>().is_ok(),
            "label '{}' doesn't look like a nice number",
            label
        );
    }
}

#[test]
fn test_tick_labels_single() {
    let axis = Axis {
        label: "x".to_string(),
        min: 5.0,
        max: 5.0,
    };
    let labels = axis.tick_labels(1);
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0], "5");
}

#[test]
fn test_format_number_millions() {
    assert_eq!(super::format_number(1_500_000.0), "1.5M");
}

#[test]
fn test_format_number_billions() {
    assert_eq!(super::format_number(2_500_000_000.0), "2.5B");
}

#[test]
fn test_format_number_trillions() {
    assert_eq!(super::format_number(1_200_000_000_000.0), "1.2T");
}

#[test]
fn test_format_number_large_value() {
    // 999,999,999,999 should be ~1000B (trailing .0 stripped)
    assert_eq!(super::format_number(999_999_999_999.0), "1000B");
}

#[test]
fn test_format_number_thousands() {
    assert_eq!(super::format_number(2500.0), "2.5k");
}

#[test]
fn test_format_number_small_integer() {
    assert_eq!(super::format_number(42.0), "42");
}

#[test]
fn test_format_number_decimal() {
    assert_eq!(super::format_number(3.7), "3.7");
}

#[test]
fn test_dedup_tick_labels() {
    let ticks = vec![
        "100".to_string(),
        "100".to_string(),
        "50".to_string(),
        "50".to_string(),
        "0".to_string(),
    ];
    let deduped = dedup_tick_labels(&ticks);
    assert_eq!(deduped, vec!["100", "", "50", "", "0"]);
}

#[test]
fn test_dedup_tick_labels_no_dups() {
    let ticks = vec!["100".to_string(), "50".to_string(), "0".to_string()];
    let deduped = dedup_tick_labels(&ticks);
    assert_eq!(deduped, ticks);
}

#[test]
fn test_dedup_tick_labels_all_same() {
    // When all tick labels are identical (e.g., axis with zero range),
    // only the first should remain; rest become empty.
    let ticks = vec![
        "0".to_string(),
        "0".to_string(),
        "0".to_string(),
        "0".to_string(),
    ];
    let deduped = dedup_tick_labels(&ticks);
    assert_eq!(deduped, vec!["0", "", "", ""]);
}

#[test]
fn test_split_y_axis_produces_two_areas() {
    let ticks = vec!["100".to_string(), "50".to_string(), "0".to_string()];
    let area = Rect::new(0, 0, 80, 24);
    let (y_area, chart_area) = split_y_axis(area, &ticks);
    // Y-axis should be small (label_width + 1)
    assert!(y_area.width <= 6);
    // Chart should take remaining space
    assert!(chart_area.width >= 74);
    assert_eq!(y_area.width + chart_area.width, area.width);
}

#[test]
fn test_render_y_axis_no_panic() {
    let ticks = vec!["100".to_string(), "50".to_string(), "0".to_string()];
    let area = Rect::new(0, 0, 6, 24);
    let mut buf = Buffer::empty(area);
    render_y_axis(&ticks, area, &mut buf, Color::DarkGray);
    // Should have rendered something
    let content: String = buf
        .content()
        .iter()
        .map(|c| c.symbol().chars().next().unwrap_or(' '))
        .collect();
    assert!(content.contains('│'));
}

#[test]
fn test_render_y_axis_frame_returns_chart_area() {
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(area);
    let chart_area = render_y_axis_frame(100.0, 5, &area, &mut buf);
    // Chart area should be smaller than original (Y-axis took some width)
    assert!(chart_area.width < area.width);
    assert!(chart_area.width >= 70); // Most of the width goes to chart
    assert_eq!(chart_area.height, area.height);
}

#[test]
fn test_render_y_axis_frame_zero_max() {
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(area);
    let chart_area = render_y_axis_frame(0.0, 5, &area, &mut buf);
    // Should not panic and still return valid area
    assert!(chart_area.width > 0);
}

#[test]
fn test_render_y_axis_frame_tight_removes_excess_headroom() {
    let area = Rect::new(0, 0, 80, 24);
    let mut buf_normal = Buffer::empty(area);
    let mut buf_tight = Buffer::empty(area);

    // max_val=4200, normal mode shows tick up to 5000
    render_y_axis_frame(4200.0, 5, &area, &mut buf_normal);
    let normal_content: String = buf_normal
        .content()
        .iter()
        .map(|c| c.symbol().chars().next().unwrap_or(' '))
        .collect();
    assert!(
        normal_content.contains("5k") || normal_content.contains("5000"),
        "Normal mode should show 5k tick"
    );

    // Tight mode should NOT show 5000 tick (it's >10% above 4200)
    render_y_axis_frame_tight(4200.0, 5, &area, &mut buf_tight);
    let tight_content: String = buf_tight
        .content()
        .iter()
        .map(|c| c.symbol().chars().next().unwrap_or(' '))
        .collect();
    assert!(
        !tight_content.contains("5k") && !tight_content.contains("5000"),
        "Tight mode should NOT show 5k tick, got:\n{}",
        tight_content.trim()
    );
    assert!(
        tight_content.contains("4k") || tight_content.contains("4000"),
        "Tight mode should show 4k as top tick"
    );
}
