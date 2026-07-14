use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Bar, BarChart as RatatuiBarChart, BarGroup, Block, Borders, Widget},
};

use super::{BarChartData, format_number};

/// Bar chart widget wrapping ratatui's BarChart with Y-axis tick labels.
pub struct BarChart<'a> {
    data: &'a BarChartData,
}

impl<'a> BarChart<'a> {
    pub fn new(data: &'a BarChartData) -> Self {
        Self { data }
    }
}

impl<'a> Widget for BarChart<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = self
            .data
            .title
            .clone()
            .unwrap_or_else(|| "Bar Chart".to_string());

        let max_val = self.data.values.iter().copied().fold(0.0_f64, f64::max);

        let chart_area = if self.data.values.is_empty() {
            area
        } else {
            let color = self.data.axis_color.unwrap_or(Color::DarkGray);
            super::render_y_axis_frame_colored(max_val, 5, &area, buf, true, color)
        };

        let bar_count = self.data.labels.len();
        let bar_width = compute_bar_width(chart_area.width, bar_count);
        let bars = build_bars(
            &self.data.labels,
            &self.data.values,
            max_val,
            self.data.show_labels,
            &self.data.series_colors,
            bar_width,
        );
        let group = BarGroup::default().bars(&bars);

        let chart = RatatuiBarChart::default()
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::TOP | Borders::RIGHT | Borders::BOTTOM),
            )
            .bar_width(bar_width)
            .bar_gap(1)
            .data(group);

        chart.render(chart_area, buf);
    }
}

/// Compute optimal bar width based on available chart width and number of bars.
fn compute_bar_width(chart_width: u16, bar_count: usize) -> u16 {
    if bar_count == 0 {
        return 3;
    }
    let inner_width = chart_width.saturating_sub(2) as usize;
    let max_width = inner_width.checked_div(bar_count).unwrap_or(3);
    max_width.saturating_sub(1).clamp(3, 20) as u16
}

/// Truncate a label to fit within max_width columns, adding "…" if needed.
fn truncate_label(label: &str, max_width: u16) -> String {
    let max = max_width as usize;
    if label.chars().count() <= max {
        return label.to_string();
    }
    if max <= 1 {
        return "…".to_string();
    }
    let truncated: String = label.chars().take(max - 1).collect();
    format!("{truncated}…")
}

/// Map a bar value to a color using a sequential gradient (matching histogram palette).
/// Palette: dark teal (low) → cyan (mid) → yellow (high).
fn value_to_bar_color(value: f64, max_val: f64) -> Color {
    if max_val <= 0.0 || value <= 0.0 {
        return Color::DarkGray;
    }
    let t = (value / max_val).clamp(0.0, 1.0);
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

/// Build ratatui Bar widgets from labels and values, scaling floats to u64.
fn build_bars<'a>(
    labels: &'a [String],
    values: &[f64],
    max_val: f64,
    show_labels: bool,
    series_colors: &[Color],
    bar_width: u16,
) -> Vec<Bar<'a>> {
    let scale_factor = if max_val > 0.0 {
        10000.0 / max_val
    } else {
        1.0
    };

    let total: f64 = if show_labels {
        values.iter().sum()
    } else {
        0.0
    };

    labels
        .iter()
        .zip(values.iter())
        .enumerate()
        .map(|(i, (label, &value))| {
            let color = if series_colors.is_empty() {
                value_to_bar_color(value, max_val)
            } else {
                series_colors[i % series_colors.len()]
            };
            let scaled = (value * scale_factor).round() as u64;
            let text = if show_labels && total > 0.0 {
                let pct = (value / total * 100.0).round() as u32;
                format!("{} ({}%)", format_number(value), pct)
            } else {
                format_number(value)
            };
            Bar::default()
                .label(Line::from(truncate_label(label, bar_width)))
                .value(scaled)
                .text_value(text)
                .style(Style::default().fg(color))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bar_chart_renders_without_panic() {
        let data = BarChartData {
            title: Some("Sales by City".to_string()),
            labels: vec![
                "Tokyo".to_string(),
                "Osaka".to_string(),
                "Nagoya".to_string(),
            ],
            values: vec![300.0, 200.0, 150.0],
            y_label: "Revenue".to_string(),
            show_labels: false,
            series_colors: vec![],
            axis_color: None,
        };

        let chart = BarChart::new(&data);
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
    fn test_bar_chart_empty_data() {
        let data = BarChartData {
            title: None,
            labels: vec![],
            values: vec![],
            y_label: "Y".to_string(),
            show_labels: false,
            series_colors: vec![],
            axis_color: None,
        };

        let chart = BarChart::new(&data);
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        chart.render(area, &mut buf);
        // Should not panic
    }

    #[test]
    fn test_bar_chart_single_bar() {
        let data = BarChartData {
            title: Some("Single".to_string()),
            labels: vec!["Only".to_string()],
            values: vec![42.0],
            y_label: "Count".to_string(),
            show_labels: false,
            series_colors: vec![],
            axis_color: None,
        };

        let chart = BarChart::new(&data);
        let area = Rect::new(0, 0, 40, 12);
        let mut buf = Buffer::empty(area);
        chart.render(area, &mut buf);
    }

    #[test]
    fn test_bar_chart_fractional_values() {
        let data = BarChartData {
            title: Some("Rates".to_string()),
            labels: vec!["A".to_string(), "B".to_string(), "C".to_string()],
            values: vec![0.75, 0.50, 0.25],
            y_label: "Rate".to_string(),
            show_labels: false,
            series_colors: vec![],
            axis_color: None,
        };

        let chart = BarChart::new(&data);
        let area = Rect::new(0, 0, 60, 16);
        let mut buf = Buffer::empty(area);
        chart.render(area, &mut buf);

        // Verify the text_value labels appear (not "0" from truncation)
        let content: String = buf
            .content()
            .iter()
            .map(|c| c.symbol().chars().next().unwrap_or(' '))
            .collect();
        // Should contain "0.75" or its formatted version, not just "0"
        assert!(
            content.contains("0.75") || content.contains("0.8"),
            "Expected fractional values to display, got:\n{}",
            content
        );
    }

    #[test]
    fn test_bar_chart_large_values_formatted() {
        let data = BarChartData {
            title: Some("Revenue".to_string()),
            labels: vec!["Tokyo".to_string(), "Osaka".to_string()],
            values: vec![1500000.0, 750000.0],
            y_label: "Revenue".to_string(),
            show_labels: false,
            series_colors: vec![],
            axis_color: None,
        };

        let chart = BarChart::new(&data);
        let area = Rect::new(0, 0, 60, 16);
        let mut buf = Buffer::empty(area);
        chart.render(area, &mut buf);

        let content: String = buf
            .content()
            .iter()
            .map(|c| c.symbol().chars().next().unwrap_or(' '))
            .collect();
        // Should use formatted numbers (1.5M or 1500000, not truncated)
        assert!(
            content.contains("1.5M") || content.contains("1500"),
            "Expected formatted large values, got:\n{}",
            content
        );
    }

    #[test]
    fn test_compute_bar_width() {
        // Zero bars
        assert_eq!(compute_bar_width(80, 0), 3);
        // One bar in 80 width → clamped to max 20
        assert_eq!(compute_bar_width(80, 1), 20);
        // Many bars → should be at least 3
        assert_eq!(compute_bar_width(20, 10), 3);
        // Normal case
        let w = compute_bar_width(80, 5);
        assert!((3..=20).contains(&w));
    }

    #[test]
    fn test_truncate_label() {
        assert_eq!(truncate_label("Tokyo", 10), "Tokyo");
        assert_eq!(truncate_label("Tokyo", 5), "Tokyo");
        assert_eq!(truncate_label("San Francisco", 5), "San …");
        assert_eq!(truncate_label("Philadelphia", 3), "Ph…");
        assert_eq!(truncate_label("AB", 1), "…");
        assert_eq!(truncate_label("X", 1), "X");
    }
}
