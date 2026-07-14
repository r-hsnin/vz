use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Bar, BarChart as RatatuiBarChart, BarGroup, Block, Borders, Widget},
};

use super::{HistogramData, compute_bins};

/// Histogram widget — uses bar chart with computed bins and Y-axis tick labels.
pub struct Histogram<'a> {
    data: &'a HistogramData,
}

impl<'a> Histogram<'a> {
    pub fn new(data: &'a HistogramData) -> Self {
        Self { data }
    }
}

impl<'a> Widget for Histogram<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let bins = compute_bins(&self.data.values, self.data.bin_count);

        let title = self
            .data
            .title
            .clone()
            .unwrap_or_else(|| "Histogram".to_string());

        let max_count = bins.iter().map(|(_, _, c)| *c).max().unwrap_or(0) as f64;

        let chart_area = if bins.is_empty() {
            area
        } else if max_count <= 10.0 {
            let y_ticks = compute_integer_ticks(max_count.ceil() as usize, 5);
            let y_ticks = super::dedup_tick_labels(&y_ticks);
            let (y_area, chart_area) = super::split_y_axis(area, &y_ticks);
            let color = self.data.axis_color.unwrap_or(Color::DarkGray);
            super::render_y_axis(&y_ticks, y_area, buf, color);
            chart_area
        } else {
            let color = self.data.axis_color.unwrap_or(Color::DarkGray);
            super::render_y_axis_frame_colored(max_count, 5, &area, buf, false, color)
        };

        render_histogram_bars(&bins, &title, chart_area, buf);
    }
}

/// Compute integer tick labels for small counts (max ≤ 10).
fn compute_integer_ticks(max_int: usize, tick_count: usize) -> Vec<String> {
    let step = (max_int / (tick_count - 1)).max(1);
    let nice_max = max_int.div_ceil(step) * step;
    let mut ticks = Vec::new();
    let mut v = nice_max;
    loop {
        ticks.push(format!("{}", v));
        if v == 0 {
            break;
        }
        v = v.saturating_sub(step);
    }
    ticks
}

/// Render histogram bars into the given area.
fn render_histogram_bars(
    bins: &[(f64, f64, usize)],
    title: &str,
    chart_area: Rect,
    buf: &mut Buffer,
) {
    let bar_count = bins.len();
    let inner_width = chart_area.width.saturating_sub(2) as usize;
    let bar_width = if bar_count == 0 {
        5u16
    } else {
        let max_width = inner_width.checked_div(bar_count).unwrap_or(5);
        max_width.saturating_sub(1).clamp(3, 12) as u16
    };

    let max_count = bins.iter().map(|(_, _, c)| *c).max().unwrap_or(0);

    let bars: Vec<Bar> = bins
        .iter()
        .map(|(start, end, count)| {
            let label = format_bin_label(*start, *end, bar_width as usize);
            Bar::default()
                .label(Line::from(label))
                .value(*count as u64)
                .style(Style::default().fg(bin_count_to_color(*count, max_count)))
        })
        .collect();

    let group = BarGroup::default().bars(&bars);

    let chart = RatatuiBarChart::default()
        .block(
            Block::default()
                .title(title.to_string())
                .borders(Borders::TOP | Borders::RIGHT | Borders::BOTTOM),
        )
        .bar_width(bar_width)
        .bar_gap(1)
        .data(group);

    chart.render(chart_area, buf);
}

/// Map a bin count to a color using a sequential gradient.
/// Palette: dark teal (low frequency) → cyan (mid) → yellow (high frequency).
fn bin_count_to_color(count: usize, max_count: usize) -> Color {
    if max_count == 0 || count == 0 {
        return Color::DarkGray;
    }
    let t = count as f64 / max_count as f64;
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

/// Format a bin label to fit within the given max width.
/// Uses abbreviated form (e.g., "1.2k") when full label would overflow.
fn format_bin_label(start: f64, end: f64, max_width: usize) -> String {
    let full = format!("{:.0}-{:.0}", start, end);
    if full.len() <= max_width {
        return full;
    }
    // Try abbreviated format
    let abbrev = format!("{}-{}", abbreviate_number(start), abbreviate_number(end));
    if abbrev.len() <= max_width {
        return abbrev;
    }
    // Fallback: just show start value abbreviated
    abbreviate_number(start)
}

/// Abbreviate a number: 1200 → "1.2k", 1500000 → "1.5M"
fn abbreviate_number(n: f64) -> String {
    let abs = n.abs();
    if abs >= 1_000_000.0 {
        format!("{:.1}M", n / 1_000_000.0)
    } else if abs >= 1_000.0 {
        format!("{:.1}k", n / 1_000.0)
    } else {
        format!("{:.0}", n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_histogram_renders_without_panic() {
        let data = HistogramData {
            title: Some("Age Distribution".to_string()),
            values: vec![
                22.0, 25.0, 28.0, 30.0, 32.0, 35.0, 38.0, 40.0, 42.0, 45.0, 50.0, 55.0,
            ],
            bin_count: 5,
            x_label: "Age".to_string(),
            axis_color: None,
        };

        let hist = Histogram::new(&data);
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        hist.render(area, &mut buf);

        let content: String = buf
            .content()
            .iter()
            .map(|c| c.symbol().chars().next().unwrap_or(' '))
            .collect();
        assert!(!content.trim().is_empty());
    }

    #[test]
    fn test_histogram_empty() {
        let data = HistogramData {
            title: None,
            values: vec![],
            bin_count: 5,
            x_label: "X".to_string(),
            axis_color: None,
        };

        let hist = Histogram::new(&data);
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        hist.render(area, &mut buf);
    }

    #[test]
    fn test_histogram_single_bin() {
        let data = HistogramData {
            title: None,
            values: vec![5.0, 5.0, 5.0, 5.0],
            bin_count: 1,
            x_label: "X".to_string(),
            axis_color: None,
        };

        let hist = Histogram::new(&data);
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        hist.render(area, &mut buf);
    }

    #[test]
    fn test_compute_integer_ticks() {
        let ticks = compute_integer_ticks(5, 5);
        // Should produce decreasing ticks from nice_max down to 0
        assert!(!ticks.is_empty());
        assert_eq!(ticks.last().unwrap(), "0");
        // First tick should be >= max_int
        let first: usize = ticks[0].parse().unwrap();
        assert!(first >= 5);
    }

    #[test]
    fn test_compute_integer_ticks_small() {
        let ticks = compute_integer_ticks(1, 5);
        assert_eq!(ticks.last().unwrap(), "0");
        let first: usize = ticks[0].parse().unwrap();
        assert!(first >= 1);
    }

    #[test]
    fn test_format_bin_label_fits() {
        assert_eq!(format_bin_label(10.0, 20.0, 10), "10-20");
        assert_eq!(format_bin_label(100.0, 200.0, 10), "100-200");
    }

    #[test]
    fn test_format_bin_label_abbreviates() {
        // "1000-2000" = 9 chars, max_width=10 → fits as-is
        assert_eq!(format_bin_label(1000.0, 2000.0, 10), "1000-2000");
        // max_width=8 → "1000-2000" (9) too long → "1.0k-2.0k" (9) too long → "1.0k"
        assert_eq!(format_bin_label(1000.0, 2000.0, 8), "1.0k");
        // "10000-20000" = 11 chars, max_width=10 → try "10.0k-20.0k" (11) → "10.0k"
        assert_eq!(format_bin_label(10000.0, 20000.0, 10), "10.0k");
        // max_width=12 → "10000-20000" (11) fits
        assert_eq!(format_bin_label(10000.0, 20000.0, 12), "10000-20000");
    }

    #[test]
    fn test_format_bin_label_very_narrow() {
        // Very narrow: just show start
        assert_eq!(format_bin_label(1000.0, 2000.0, 3), "1.0k");
    }

    #[test]
    fn test_abbreviate_number() {
        assert_eq!(abbreviate_number(500.0), "500");
        assert_eq!(abbreviate_number(1500.0), "1.5k");
        assert_eq!(abbreviate_number(2_500_000.0), "2.5M");
    }

    #[test]
    fn test_bin_count_to_color_gradient() {
        let low = bin_count_to_color(1, 10);
        let high = bin_count_to_color(10, 10);
        let zero = bin_count_to_color(0, 10);
        assert_eq!(zero, Color::DarkGray);
        assert_ne!(low, high);
        // High frequency should be warm (yellow-ish: high R+G, low B)
        if let Color::Rgb(r, _g, b) = high {
            assert!(r > 200);
            assert!(b < 50);
        } else {
            panic!("Expected Rgb color for high count");
        }
    }

    #[test]
    fn test_bin_count_to_color_zero_max() {
        assert_eq!(bin_count_to_color(5, 0), Color::DarkGray);
    }

    #[test]
    fn test_bin_count_to_color_mid() {
        let mid = bin_count_to_color(5, 10);
        // Mid-range should be a cyan-ish color (high G, moderate B)
        if let Color::Rgb(_r, g, _b) = mid {
            assert!(g > 200);
        } else {
            panic!("Expected Rgb color for mid count");
        }
    }
}
