use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

pub mod bar;
pub mod heatmap;
pub mod histogram;
pub mod line;
pub mod nice_numbers;
pub mod scatter;

/// Shared color palette for multi-series charts.
pub const SERIES_COLORS: &[Color] = &[
    Color::Cyan,
    Color::Yellow,
    Color::Green,
    Color::Magenta,
    Color::Red,
    Color::Blue,
];

/// A data series for rendering.
#[derive(Debug, Clone, PartialEq)]
pub struct Series {
    pub name: String,
    pub data: Vec<(f64, f64)>,
}

/// Axis metadata for rendering.
#[derive(Debug, Clone, PartialEq)]
pub struct Axis {
    pub label: String,
    pub min: f64,
    pub max: f64,
}

impl Axis {
    pub fn from_data(label: &str, values: &[f64]) -> Self {
        if values.is_empty() {
            return Self {
                label: label.to_string(),
                min: 0.0,
                max: 1.0,
            };
        }

        let data_min = values.iter().copied().fold(f64::INFINITY, f64::min);
        let data_max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);

        let scale = nice_numbers::nice_scale(data_min, data_max, 5);

        Self {
            label: label.to_string(),
            min: scale.min,
            max: scale.max,
        }
    }

    /// Normalize a value to [0.0, 1.0] range within this axis.
    pub fn normalize(&self, value: f64) -> f64 {
        if (self.max - self.min).abs() < f64::EPSILON {
            return 0.5;
        }
        (value - self.min) / (self.max - self.min)
    }

    /// Generate tick label strings using nice numbers.
    /// Returns labels at evenly spaced "nice" positions across the axis range.
    pub fn tick_labels(&self, count: usize) -> Vec<String> {
        if count <= 1 {
            return vec![format_number(self.min)];
        }
        // Use nice_scale to compute nice tick positions within the existing range
        let scale = nice_numbers::nice_scale(self.min, self.max, count);
        scale
            .tick_values()
            .iter()
            .map(|&val| format_number(val))
            .collect()
    }
}

/// Chart configuration for rendering.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartConfig {
    pub title: Option<String>,
    pub x_axis: Axis,
    pub y_axis: Axis,
    pub series: Vec<Series>,
    /// Optional string labels for the X axis (e.g., date strings for temporal data).
    /// When set, these replace numeric tick labels on the X axis.
    pub x_labels: Option<Vec<String>>,
    /// Color palette for series (from theme). Falls back to SERIES_COLORS if empty.
    pub series_colors: Vec<Color>,
}

/// Labels for a bar chart (categorical x-axis).
#[derive(Debug, Clone, PartialEq)]
pub struct BarChartData {
    pub title: Option<String>,
    pub labels: Vec<String>,
    pub values: Vec<f64>,
    pub y_label: String,
    /// Show value + percentage labels on bars.
    pub show_labels: bool,
    /// Color palette for bars (from theme). Falls back to SERIES_COLORS if empty.
    pub series_colors: Vec<Color>,
}

/// Data for a histogram.
#[derive(Debug, Clone, PartialEq)]
pub struct HistogramData {
    pub title: Option<String>,
    pub values: Vec<f64>,
    pub bin_count: usize,
    pub x_label: String,
}

/// Data for a heatmap (count matrix of two categorical columns).
#[derive(Debug, Clone, PartialEq)]
pub struct HeatmapData {
    pub title: Option<String>,
    /// Row labels (Y axis categories).
    pub row_labels: Vec<String>,
    /// Column labels (X axis categories).
    pub col_labels: Vec<String>,
    /// Count matrix: `counts[row][col]`.
    pub counts: Vec<Vec<usize>>,
    /// Maximum count value (for color scaling).
    pub max_count: usize,
}

/// Format a number concisely for tick labels.
fn format_number(val: f64) -> String {
    let abs = val.abs();
    if abs >= 1_000_000_000_000.0 {
        format!("{:.1}T", val / 1_000_000_000_000.0)
    } else if abs >= 1_000_000_000.0 {
        format!("{:.1}B", val / 1_000_000_000.0)
    } else if abs >= 1_000_000.0 {
        format!("{:.1}M", val / 1_000_000.0)
    } else if abs >= 1_000.0 {
        format!("{:.1}k", val / 1_000.0)
    } else if abs < 0.01 && abs > 0.0 {
        format!("{:.2e}", val)
    } else if (val - val.round()).abs() < f64::EPSILON {
        format!("{:.0}", val)
    } else {
        format!("{:.1}", val)
    }
}

/// Public re-export of format_number for use in submodules.
pub fn format_number_pub(val: f64) -> String {
    format_number(val)
}

/// Compute histogram bins from raw values.
pub fn compute_bins(values: &[f64], bin_count: usize) -> Vec<(f64, f64, usize)> {
    if values.is_empty() || bin_count == 0 {
        return vec![];
    }

    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    if (max - min).abs() < f64::EPSILON {
        return vec![(min, max, values.len())];
    }

    let bin_width = (max - min) / bin_count as f64;
    let mut bins: Vec<(f64, f64, usize)> = (0..bin_count)
        .map(|i| {
            let start = min + i as f64 * bin_width;
            let end = start + bin_width;
            (start, end, 0)
        })
        .collect();

    for &v in values {
        let idx = ((v - min) / bin_width).floor() as usize;
        let idx = idx.min(bin_count - 1); // clamp last value
        bins[idx].2 += 1;
    }

    bins
}

/// Deduplicate consecutive identical tick labels (replace dups with empty string).
pub fn dedup_tick_labels(ticks: &[String]) -> Vec<String> {
    ticks
        .iter()
        .enumerate()
        .map(|(i, label)| {
            if i > 0 && *label == ticks[i - 1] {
                String::new()
            } else {
                label.clone()
            }
        })
        .collect()
}

/// Render Y-axis tick labels in the given area.
/// `y_ticks` should be ordered from top (max) to bottom (min).
pub fn render_y_axis(y_ticks: &[String], area: Rect, buf: &mut Buffer) {
    let y_label_width = y_ticks.iter().map(|s| s.len()).max().unwrap_or(3).max(3) as u16;
    let chart_inner_height = area.height.saturating_sub(3) as usize;
    if chart_inner_height == 0 {
        return;
    }

    let tick_count = y_ticks.len();
    let tick_positions: Vec<usize> = (0..tick_count)
        .map(|i| {
            if tick_count <= 1 {
                0
            } else {
                (i as f64 * (chart_inner_height.saturating_sub(1)) as f64 / (tick_count - 1) as f64)
                    .round() as usize
            }
        })
        .collect();

    let mut lines: Vec<Line> = Vec::with_capacity(area.height as usize);
    lines.push(Line::from(""));

    for row in 0..chart_inner_height {
        if let Some(tick_idx) = tick_positions.iter().position(|&pos| pos == row) {
            let label = &y_ticks[tick_idx];
            lines.push(Line::from(Span::styled(
                format!("{:>width$}│", label, width = y_label_width as usize),
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                format!("{:>width$}│", "", width = y_label_width as usize),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(""));

    Paragraph::new(lines).render(area, buf);
}

/// Split an area into Y-axis label area (left) and chart area (right).
/// Returns (y_axis_area, chart_area).
pub fn split_y_axis(area: Rect, y_ticks: &[String]) -> (Rect, Rect) {
    let y_label_width = y_ticks.iter().map(|s| s.len()).max().unwrap_or(3).max(3) as u16;
    let chunks = Layout::horizontal([Constraint::Length(y_label_width + 1), Constraint::Min(10)])
        .split(area);
    (chunks[0], chunks[1])
}

/// Compute Y-axis ticks, render the axis, and return the remaining chart area.
/// This encapsulates the shared pattern used by bar and histogram charts:
/// nice_scale → format ticks → dedup → split → render → return chart_area.
pub fn render_y_axis_frame(max_val: f64, tick_count: usize, area: &Rect, buf: &mut Buffer) -> Rect {
    render_y_axis_frame_inner(max_val, tick_count, area, buf, false)
}

/// Render Y-axis frame with tight scaling (max stays close to data max).
/// Used for bar charts where wasted headroom reduces readability.
pub fn render_y_axis_frame_tight(
    max_val: f64,
    tick_count: usize,
    area: &Rect,
    buf: &mut Buffer,
) -> Rect {
    render_y_axis_frame_inner(max_val, tick_count, area, buf, true)
}

fn render_y_axis_frame_inner(
    max_val: f64,
    tick_count: usize,
    area: &Rect,
    buf: &mut Buffer,
    tight: bool,
) -> Rect {
    let scale = nice_numbers::nice_scale(0.0, max_val, tick_count);

    // Generate ticks. In tight mode, omit the top tick if it creates excessive
    // headroom above the actual data max, improving bar chart readability.
    let mut tick_vals: Vec<f64> = Vec::new();
    let mut val = scale.min;
    while val <= scale.max + scale.tick_spacing * 0.01 {
        tick_vals.push(val);
        val += scale.tick_spacing;
    }

    if tight && max_val > 0.0 && tick_vals.len() > 2 {
        // If the top tick is significantly above data max (> 10% headroom),
        // remove it so the chart area scales tighter to the actual data.
        if let Some(&top) = tick_vals.last() {
            let headroom = (top - max_val) / max_val;
            if headroom > 0.10 {
                tick_vals.pop();
            }
        }
    }

    let y_ticks: Vec<String> = tick_vals
        .iter()
        .rev()
        .map(|&v| format_number_pub(v))
        .collect();
    let y_ticks = dedup_tick_labels(&y_ticks);

    let (y_area, chart_area) = split_y_axis(*area, &y_ticks);
    render_y_axis(&y_ticks, y_area, buf);
    chart_area
}

/// Unified chart data enum — one variant per chart kind.
/// Callers build the appropriate variant; `render_chart_data` dispatches to the correct widget.
#[derive(Debug, Clone, PartialEq)]
pub enum ChartData {
    Line(ChartConfig),
    Scatter(ChartConfig),
    Bar(BarChartData),
    Histogram(HistogramData),
    Heatmap(HeatmapData),
}

impl ChartData {
    /// Set the title on any chart data variant.
    pub fn set_title(&mut self, title: String) {
        match self {
            ChartData::Line(c) | ChartData::Scatter(c) => c.title = Some(title),
            ChartData::Bar(d) => d.title = Some(title),
            ChartData::Histogram(d) => d.title = Some(title),
            ChartData::Heatmap(d) => d.title = Some(title),
        }
    }
}

/// Render a `ChartData` value into the given buffer area.
/// This is the single dispatch point for all chart types across all modes.
pub fn render_chart_data(data: &ChartData, area: Rect, buf: &mut Buffer) {
    match data {
        ChartData::Line(config) => line::LineChart::new(config).render(area, buf),
        ChartData::Scatter(config) => scatter::ScatterPlot::new(config).render(area, buf),
        ChartData::Bar(bar_data) => bar::BarChart::new(bar_data).render(area, buf),
        ChartData::Histogram(hist_data) => histogram::Histogram::new(hist_data).render(area, buf),
        ChartData::Heatmap(heat_data) => heatmap::HeatmapChart::new(heat_data).render(area, buf),
    }
}

/// Widget wrapper for `ChartData` — use with `frame.render_widget(ChartWidget(data), area)`.
pub struct ChartWidget<'a>(pub &'a ChartData);

impl Widget for ChartWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        render_chart_data(self.0, area, buf);
    }
}

#[cfg(test)]
mod tests {
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
        // 999,999,999,999 should be ~1.0T
        assert_eq!(super::format_number(999_999_999_999.0), "1000.0B");
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
        render_y_axis(&ticks, area, &mut buf);
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
            normal_content.contains("5.0k") || normal_content.contains("5000"),
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
            !tight_content.contains("5.0k") && !tight_content.contains("5000"),
            "Tight mode should NOT show 5k tick, got:\n{}",
            tight_content.trim()
        );
        assert!(
            tight_content.contains("4.0k") || tight_content.contains("4000"),
            "Tight mode should show 4k as top tick"
        );
    }
}
