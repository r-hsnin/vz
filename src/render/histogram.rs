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
            super::render_y_axis(&y_ticks, y_area, buf, Color::DarkGray);
            chart_area
        } else {
            super::render_y_axis_frame(max_count, 5, &area, buf)
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

    let bars: Vec<Bar> = bins
        .iter()
        .map(|(start, end, count)| {
            let label = format!("{:.0}-{:.0}", start, end);
            Bar::default()
                .label(Line::from(label))
                .value(*count as u64)
                .style(Style::default().fg(Color::Cyan))
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
}
