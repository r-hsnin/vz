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

        let (data_min, data_max) = crate::util::min_max(values).unwrap_or((0.0, 1.0));

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
    /// Color for axis lines/ticks. Falls back to DarkGray if not set.
    pub axis_color: Option<Color>,
    /// Color for axis label text. Falls back to axis_color then DarkGray.
    pub label_color: Option<Color>,
}

impl ChartConfig {
    /// Apply theme colors to this config (series, axis, label).
    pub fn apply_theme(&mut self, theme: &crate::theme::Theme) {
        self.series_colors = theme.series_colors.clone();
        self.axis_color = Some(theme.axis_color);
        self.label_color = Some(theme.label_color);
    }
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
    /// Color for axis lines (from theme). Falls back to DarkGray if not set.
    pub axis_color: Option<Color>,
}

/// Data for a histogram.
#[derive(Debug, Clone, PartialEq)]
pub struct HistogramData {
    pub title: Option<String>,
    pub values: Vec<f64>,
    pub bin_count: usize,
    pub x_label: String,
    /// Color for axis lines (from theme). Falls back to DarkGray if not set.
    pub axis_color: Option<Color>,
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
pub(crate) fn format_number(val: f64) -> String {
    let abs = val.abs();
    if abs >= 1_000_000_000_000.0 {
        format_with_suffix(val / 1_000_000_000_000.0, "T")
    } else if abs >= 1_000_000_000.0 {
        format_with_suffix(val / 1_000_000_000.0, "B")
    } else if abs >= 1_000_000.0 {
        format_with_suffix(val / 1_000_000.0, "M")
    } else if abs >= 1_000.0 {
        format_with_suffix(val / 1_000.0, "k")
    } else if abs < 0.01 && abs > 0.0 {
        format!("{:.2e}", val)
    } else if (val - val.round()).abs() < f64::EPSILON {
        format!("{:.0}", val)
    } else {
        format!("{:.1}", val)
    }
}

/// Format a scaled value with a suffix, removing trailing ".0" for clean integers.
fn format_with_suffix(val: f64, suffix: &str) -> String {
    let s = format!("{:.1}", val);
    let trimmed = s.strip_suffix(".0").unwrap_or(&s);
    format!("{trimmed}{suffix}")
}

/// Map a normalized value t ∈ [0.0, 1.0] to a sequential gradient color.
/// Palette: dark teal (low) → cyan (mid) → yellow (high).
/// Values outside [0.0, 1.0] are clamped.
pub fn gradient_color(t: f64) -> Color {
    let t = t.clamp(0.0, 1.0);
    let (r, g, b) = if t < 0.5 {
        let s = t * 2.0;
        (
            (20.0 + s * 10.0) as u8,
            (60.0 + s * 195.0) as u8,
            (120.0 + s * 135.0) as u8,
        )
    } else {
        let s = (t - 0.5) * 2.0;
        ((30.0 + s * 225.0) as u8, 255, (255.0 - s * 255.0) as u8)
    };
    Color::Rgb(r, g, b)
}

/// Compute histogram bins from raw values.
pub fn compute_bins(values: &[f64], bin_count: usize) -> Vec<(f64, f64, usize)> {
    if values.is_empty() || bin_count == 0 {
        return vec![];
    }

    let (min, max) = crate::util::min_max(values).unwrap_or((0.0, 0.0));

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
pub fn render_y_axis(y_ticks: &[String], area: Rect, buf: &mut Buffer, color: Color) {
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
                Style::default().fg(color),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                format!("{:>width$}│", "", width = y_label_width as usize),
                Style::default().fg(color),
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
    render_y_axis_frame_inner(max_val, tick_count, area, buf, false, Color::DarkGray)
}

/// Render Y-axis frame with tight scaling (max stays close to data max).
/// Used for bar charts where wasted headroom reduces readability.
pub fn render_y_axis_frame_tight(
    max_val: f64,
    tick_count: usize,
    area: &Rect,
    buf: &mut Buffer,
) -> Rect {
    render_y_axis_frame_inner(max_val, tick_count, area, buf, true, Color::DarkGray)
}

/// Render Y-axis frame with a specific axis color.
pub fn render_y_axis_frame_colored(
    max_val: f64,
    tick_count: usize,
    area: &Rect,
    buf: &mut Buffer,
    tight: bool,
    axis_color: Color,
) -> Rect {
    render_y_axis_frame_inner(max_val, tick_count, area, buf, tight, axis_color)
}

fn render_y_axis_frame_inner(
    max_val: f64,
    tick_count: usize,
    area: &Rect,
    buf: &mut Buffer,
    tight: bool,
    axis_color: Color,
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

    let y_ticks: Vec<String> = tick_vals.iter().rev().map(|&v| format_number(v)).collect();
    let y_ticks = dedup_tick_labels(&y_ticks);

    let (y_area, chart_area) = split_y_axis(*area, &y_ticks);
    render_y_axis(&y_ticks, y_area, buf, axis_color);
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
mod tests;
