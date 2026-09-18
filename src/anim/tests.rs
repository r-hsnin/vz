//! RED-first tests for anim gating, easing, frame interpolation, player.

use super::*;
use crate::chart::selector::ChartType;
use crate::cli::MotionArg;
use crate::render::{BarChartData, ChartConfig, ChartData};

// --- should_animate ---

#[test]
fn test_should_animate_off_never_animates_even_on_tty() {
    let cfg = MotionConfig::new(MotionArg::Off, 12, 12);
    assert!(!should_animate(&cfg, true, false));
}

#[test]
fn test_should_animate_auto_requires_tty() {
    let cfg = MotionConfig::new(MotionArg::Auto, 12, 12);
    assert!(!should_animate(&cfg, false, false));
    assert!(should_animate(&cfg, true, false));
}

#[test]
fn test_should_animate_no_color_disables() {
    let cfg = MotionConfig::new(MotionArg::Auto, 12, 12);
    assert!(!should_animate(&cfg, true, true));
}

// --- resolve_effect ---

#[test]
fn test_resolve_effect_auto_per_chart_type() {
    assert_eq!(
        resolve_effect(MotionArg::Auto, ChartType::Bar),
        Some(Effect::Grow)
    );
    assert_eq!(
        resolve_effect(MotionArg::Auto, ChartType::Line),
        Some(Effect::Draw)
    );
    assert_eq!(
        resolve_effect(MotionArg::Auto, ChartType::Scatter),
        Some(Effect::Draw)
    );
    assert_eq!(resolve_effect(MotionArg::Auto, ChartType::Histogram), None);
    assert_eq!(resolve_effect(MotionArg::Auto, ChartType::Heatmap), None);
}

#[test]
fn test_resolve_effect_off_is_none() {
    assert_eq!(resolve_effect(MotionArg::Off, ChartType::Bar), None);
}

#[test]
fn test_resolve_effect_explicit_mismatch_is_none() {
    assert_eq!(resolve_effect(MotionArg::Grow, ChartType::Line), None);
    assert_eq!(resolve_effect(MotionArg::Draw, ChartType::Bar), None);
}

// --- ease_out_cubic ---

#[test]
fn test_ease_out_cubic_boundaries() {
    assert!((ease_out_cubic(0.0)).abs() < f64::EPSILON);
    assert!((ease_out_cubic(1.0) - 1.0).abs() < f64::EPSILON);
}

#[test]
fn test_ease_out_cubic_fast_start() {
    // ease-out: halfway through time, more than half the progress is done.
    assert!(ease_out_cubic(0.5) > 0.5);
}

// --- MotionConfig clamping ---

#[test]
fn test_motion_config_clamps_ranges() {
    let cfg = MotionConfig::new(MotionArg::Auto, 0, 100);
    assert_eq!(cfg.fps, 1);
    assert_eq!(cfg.frames, 30);
}

// --- frames ---

fn bar_chart(values: Vec<f64>) -> ChartData {
    ChartData::Bar(BarChartData {
        title: None,
        labels: values.iter().map(|v| format!("{v}")).collect(),
        values,
        y_label: "y".to_string(),
        show_labels: false,
        series_colors: vec![],
        axis_color: None,
        y_max_hint: None,
    })
}

fn line_chart(points: Vec<(f64, f64)>) -> ChartData {
    ChartData::Line(ChartConfig {
        title: None,
        x_axis: crate::render::Axis::from_data("x", &[0.0, 4.0]),
        y_axis: crate::render::Axis::from_data("y", &[0.0, 10.0]),
        series: vec![crate::render::Series {
            name: "s".to_string(),
            data: points,
        }],
        x_labels: None,
        series_colors: vec![],
        axis_color: None,
        label_color: None,
    })
}

#[test]
fn test_build_frames_none_effect_returns_final_only() {
    let chart = bar_chart(vec![10.0, 20.0]);
    let frames = build_frames(&chart, None, 12);
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0], chart);
}

#[test]
fn test_build_frames_grow_last_equals_final() {
    let chart = bar_chart(vec![10.0, 20.0, 30.0]);
    let frames = build_frames(&chart, Some(Effect::Grow), 5);
    assert_eq!(frames.len(), 5);
    assert_eq!(frames[4], chart);
}

#[test]
fn test_build_frames_grow_monotonic_and_pinned_axis() {
    let chart = bar_chart(vec![10.0, 20.0]);
    let frames = build_frames(&chart, Some(Effect::Grow), 4);
    let first = match &frames[0] {
        ChartData::Bar(d) => d,
        _ => panic!("expected bar frame"),
    };
    assert!(first.values.iter().all(|v| *v == 0.0));
    // Axis pinned: intermediate frames carry the final max; the last frame
    // is the untouched final chart (its own max already pins the axis).
    for f in &frames[..frames.len() - 1] {
        match f {
            ChartData::Bar(d) => assert_eq!(d.y_max_hint, Some(20.0)),
            _ => panic!("expected bar frame"),
        }
    }
    // Monotonic growth per bar.
    for w in frames.windows(2) {
        let (a, b) = match (&w[0], &w[1]) {
            (ChartData::Bar(a), ChartData::Bar(b)) => (a, b),
            _ => panic!("expected bar frames"),
        };
        for (x, y) in a.values.iter().zip(b.values.iter()) {
            assert!(y >= x);
        }
    }
}

#[test]
fn test_build_frames_draw_reveals_prefix_and_keeps_axes() {
    let chart = line_chart(vec![(0.0, 1.0), (1.0, 2.0), (2.0, 3.0), (3.0, 4.0)]);
    let frames = build_frames(&chart, Some(Effect::Draw), 4);
    assert_eq!(frames.len(), 4);
    assert_eq!(frames[3], chart);
    let first_len = match &frames[0] {
        ChartData::Line(c) => c.series[0].data.len(),
        _ => panic!("expected line frame"),
    };
    assert_eq!(first_len, 1);
    // Axes never move between frames.
    for f in &frames {
        match f {
            ChartData::Line(c) => {
                assert_eq!(
                    c.x_axis,
                    match &chart {
                        ChartData::Line(o) => o.x_axis.clone(),
                        _ => unreachable!(),
                    }
                );
                assert_eq!(
                    c.y_axis,
                    match &chart {
                        ChartData::Line(o) => o.y_axis.clone(),
                        _ => unreachable!(),
                    }
                );
            }
            _ => panic!("expected line frame"),
        }
    }
}

#[test]
fn test_build_frames_mismatch_returns_final_only() {
    let chart = line_chart(vec![(0.0, 1.0)]);
    let frames = build_frames(&chart, Some(Effect::Grow), 5);
    assert_eq!(frames, vec![chart]);
}

// --- player ---

#[test]
fn test_play_frames_empty_writes_nothing() {
    let mut out = Vec::new();
    let area = ratatui::layout::Rect::new(0, 0, 20, 5);
    let n = player::play_frames(
        &mut out,
        &[],
        area,
        std::time::Duration::ZERO,
        |_, _| vec![],
    )
    .unwrap();
    assert_eq!(n, 0);
    assert!(out.is_empty());
}

#[test]
fn test_play_frames_single_frame_no_cursor_up() {
    let mut out = Vec::new();
    let area = ratatui::layout::Rect::new(0, 0, 20, 2);
    let frames = vec![bar_chart(vec![5.0])];
    let n = player::play_frames(
        &mut out,
        &frames,
        area,
        std::time::Duration::ZERO,
        |_, _| vec!["row1".to_string(), "row2".to_string()],
    )
    .unwrap();
    assert_eq!(n, 1);
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("row1"));
    assert!(
        !text.contains("\x1b[2A"),
        "single frame must not move cursor"
    );
}

#[test]
fn test_play_frames_multi_frame_rewinds_by_area_height() {
    let mut out = Vec::new();
    let area = ratatui::layout::Rect::new(0, 0, 20, 3);
    let final_chart = bar_chart(vec![5.0]);
    let frames = build_frames(&final_chart, Some(Effect::Grow), 3);
    let n = player::play_frames(
        &mut out,
        &frames,
        area,
        std::time::Duration::ZERO,
        |_, _| vec!["a".to_string(), "b".to_string(), "c".to_string()],
    )
    .unwrap();
    assert_eq!(n, 3);
    let text = String::from_utf8(out).unwrap();
    assert!(
        text.contains("\x1b[3A"),
        "rewind must match area height, got: {text:?}"
    );
}
