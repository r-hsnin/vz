use super::*;
use crate::render::{Axis, Series};

#[test]
fn test_line_chart_renders_without_panic() {
    let config = ChartConfig {
        title: Some("Revenue Over Time".to_string()),
        x_axis: Axis {
            label: "Month".to_string(),
            min: 0.0,
            max: 12.0,
        },
        y_axis: Axis {
            label: "Revenue".to_string(),
            min: 0.0,
            max: 1000.0,
        },
        series: vec![Series {
            name: "Sales".to_string(),
            data: vec![(1.0, 100.0), (2.0, 200.0), (3.0, 350.0)],
        }],
        x_labels: None,
        series_colors: vec![],
        axis_color: None,
        label_color: None,
    };

    let chart = XYChart::new(&config, XYMode::Line);
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(area);
    chart.render(area, &mut buf);

    let content: String = buf
        .content()
        .iter()
        .map(|c| c.symbol().chars().next().unwrap_or(' '))
        .collect();
    assert!(!content.trim().is_empty());
}

#[test]
fn test_scatter_renders_without_panic() {
    let config = ChartConfig {
        title: Some("Height vs Weight".to_string()),
        x_axis: Axis {
            label: "Height (cm)".to_string(),
            min: 150.0,
            max: 200.0,
        },
        y_axis: Axis {
            label: "Weight (kg)".to_string(),
            min: 40.0,
            max: 120.0,
        },
        series: vec![Series {
            name: "People".to_string(),
            data: vec![
                (165.0, 60.0),
                (170.0, 65.0),
                (175.0, 72.0),
                (180.0, 80.0),
                (185.0, 85.0),
            ],
        }],
        x_labels: None,
        series_colors: vec![],
        axis_color: None,
        label_color: None,
    };

    let chart = ScatterPlot::new(&config);
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(area);
    chart.render(area, &mut buf);

    let content: String = buf
        .content()
        .iter()
        .map(|c| c.symbol().chars().next().unwrap_or(' '))
        .collect();
    assert!(!content.trim().is_empty());
}

#[test]
fn test_line_chart_multiple_series() {
    let config = ChartConfig {
        title: None,
        x_axis: Axis {
            label: "X".to_string(),
            min: 0.0,
            max: 5.0,
        },
        y_axis: Axis {
            label: "Y".to_string(),
            min: 0.0,
            max: 100.0,
        },
        series: vec![
            Series {
                name: "A".to_string(),
                data: vec![(0.0, 10.0), (1.0, 50.0)],
            },
            Series {
                name: "B".to_string(),
                data: vec![(0.0, 30.0), (1.0, 80.0)],
            },
        ],
        x_labels: None,
        series_colors: vec![],
        axis_color: None,
        label_color: None,
    };

    let chart = XYChart::new(&config, XYMode::Line);
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(area);
    chart.render(area, &mut buf);
}

#[test]
fn test_single_point_series_uses_dot_marker() {
    let single = dataset_spec(XYMode::Line, 1);
    assert_eq!(single.marker, Marker::Dot);
    assert_eq!(single.graph_type, GraphType::Scatter);

    let multi = dataset_spec(XYMode::Line, 3);
    assert_eq!(multi.marker, Marker::Braille);
    assert_eq!(multi.graph_type, GraphType::Line);

    let empty = dataset_spec(XYMode::Line, 0);
    assert_eq!(empty.marker, Marker::Dot);
    assert_eq!(empty.graph_type, GraphType::Scatter);
}

#[test]
fn test_scatter_uses_braille_for_multi_point() {
    let single = dataset_spec(XYMode::Scatter, 1);
    assert_eq!(single.marker, Marker::Dot);
    assert_eq!(single.graph_type, GraphType::Scatter);

    let multi = dataset_spec(XYMode::Scatter, 3);
    assert_eq!(multi.marker, Marker::Braille);
    assert_eq!(multi.graph_type, GraphType::Scatter);

    let empty = dataset_spec(XYMode::Scatter, 0);
    assert_eq!(empty.marker, Marker::Dot);
    assert_eq!(empty.graph_type, GraphType::Scatter);
}

#[test]
fn test_scatter_empty_data() {
    let config = ChartConfig {
        title: None,
        x_axis: Axis {
            label: "X".to_string(),
            min: 0.0,
            max: 1.0,
        },
        y_axis: Axis {
            label: "Y".to_string(),
            min: 0.0,
            max: 1.0,
        },
        series: vec![Series {
            name: "Empty".to_string(),
            data: vec![],
        }],
        x_labels: None,
        series_colors: vec![],
        axis_color: None,
        label_color: None,
    };

    let chart = ScatterPlot::new(&config);
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(area);
    chart.render(area, &mut buf);
}

#[test]
fn test_single_point_line_renders_without_panic() {
    let config = ChartConfig {
        title: Some("Test".to_string()),
        x_axis: Axis {
            label: "X".to_string(),
            min: 0.0,
            max: 10.0,
        },
        y_axis: Axis {
            label: "Y".to_string(),
            min: 0.0,
            max: 100.0,
        },
        series: vec![
            Series {
                name: "Multi".to_string(),
                data: vec![(1.0, 20.0), (5.0, 60.0), (9.0, 80.0)],
            },
            Series {
                name: "Single".to_string(),
                data: vec![(3.0, 50.0)],
            },
        ],
        x_labels: None,
        series_colors: vec![],
        axis_color: None,
        label_color: None,
    };

    let chart = XYChart::new(&config, XYMode::Line);
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(area);
    chart.render(area, &mut buf);
}

#[test]
fn test_scatter_multi_series() {
    let config = ChartConfig {
        title: Some("Multi-Series Scatter".to_string()),
        x_axis: Axis {
            label: "X".to_string(),
            min: 0.0,
            max: 100.0,
        },
        y_axis: Axis {
            label: "Y".to_string(),
            min: 0.0,
            max: 100.0,
        },
        series: vec![
            Series {
                name: "Group A".to_string(),
                data: vec![(10.0, 20.0), (30.0, 40.0), (50.0, 60.0)],
            },
            Series {
                name: "Group B".to_string(),
                data: vec![(15.0, 80.0), (45.0, 10.0), (70.0, 50.0)],
            },
        ],
        x_labels: None,
        series_colors: vec![],
        axis_color: None,
        label_color: None,
    };

    let chart = ScatterPlot::new(&config);
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(area);
    chart.render(area, &mut buf);

    let content: String = buf
        .content()
        .iter()
        .map(|c| c.symbol().chars().next().unwrap_or(' '))
        .collect();
    // Verify chart rendered something (not blank)
    assert!(!content.trim().is_empty());
    // Verify title is rendered
    assert!(content.contains("Multi-Series"));
}

#[test]
fn test_scatter_negative_coordinates() {
    let config = ChartConfig {
        title: None,
        x_axis: Axis {
            label: "X".to_string(),
            min: -50.0,
            max: 50.0,
        },
        y_axis: Axis {
            label: "Y".to_string(),
            min: -100.0,
            max: 100.0,
        },
        series: vec![Series {
            name: "Negatives".to_string(),
            data: vec![(-30.0, -50.0), (0.0, 0.0), (30.0, 50.0)],
        }],
        x_labels: None,
        series_colors: vec![],
        axis_color: None,
        label_color: None,
    };

    let chart = ScatterPlot::new(&config);
    let area = Rect::new(0, 0, 60, 16);
    let mut buf = Buffer::empty(area);
    chart.render(area, &mut buf);

    // Should render without panic and produce output
    let content: String = buf
        .content()
        .iter()
        .map(|c| c.symbol().chars().next().unwrap_or(' '))
        .collect();
    assert!(!content.trim().is_empty());
}
