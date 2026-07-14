use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

use super::HeatmapData;

/// A heatmap chart widget rendering a count matrix with color intensity.
pub struct HeatmapChart<'a> {
    data: &'a HeatmapData,
}

impl<'a> HeatmapChart<'a> {
    pub fn new(data: &'a HeatmapData) -> Self {
        Self { data }
    }
}

/// Map a count value to a color intensity using a perceptually uniform gradient.
/// Palette: dark teal (low) → cyan (mid) → yellow (high).
fn count_to_color(count: usize, max_count: usize) -> Color {
    if max_count == 0 || count == 0 {
        return Color::DarkGray;
    }
    let t = count as f64 / max_count as f64;
    // 3-stop sequential gradient optimized for dark terminals
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

/// Format a count for display in a cell.
fn format_count(count: usize) -> String {
    if count == 0 {
        "·".to_string()
    } else {
        count.to_string()
    }
}

/// Computed layout for heatmap rendering.
struct HeatmapLayout {
    chart_x: u16,
    chart_y: u16,
    chart_height: u16,
    row_label_width: u16,
    cell_width: u16,
    cell_height: u16,
    bound_x: u16,
    bound_y: u16,
}

impl Widget for HeatmapChart<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 3 {
            return;
        }
        let data = self.data;
        if data.row_labels.is_empty() || data.col_labels.is_empty() {
            return;
        }

        let row_label_width = data
            .row_labels
            .iter()
            .map(|l| l.len())
            .max()
            .unwrap_or(1)
            .min(12) as u16;
        let title_height: u16 = if data.title.is_some() { 1 } else { 0 };

        let chart_x = area.x + row_label_width + 1;
        let chart_y = area.y + title_height;
        let chart_width = area.width.saturating_sub(row_label_width + 1);
        let chart_height = area.height.saturating_sub(title_height + 1);

        if chart_width < 2 || chart_height < 2 {
            return;
        }

        let layout = HeatmapLayout {
            chart_x,
            chart_y,
            chart_height,
            row_label_width,
            cell_width: (chart_width / data.col_labels.len() as u16).max(3),
            cell_height: (chart_height / data.row_labels.len() as u16).max(1),
            bound_x: area.x + area.width,
            bound_y: area.y + area.height,
        };

        if let Some(ref title) = data.title {
            let title_str: String = title.chars().take(area.width as usize).collect();
            buf.set_string(area.x, area.y, &title_str, Style::default());
        }

        render_legend(data.max_count, area, buf);
        render_labels(data, &layout, area.x, buf);
        render_cells(data, &layout, buf);
    }
}

/// Render a compact color scale legend on the title row (right-aligned).
fn render_legend(max_count: usize, area: Rect, buf: &mut Buffer) {
    if area.width < 20 || area.height < 1 {
        return;
    }
    let max_str = max_count.to_string();
    // Legend: "0 ▁▂▃▅▇ {max}"
    let steps = 5;
    let legend_width = 2 + steps + 1 + max_str.len() as u16; // "0 " + blocks + " " + max
    let start_x = area.x + area.width.saturating_sub(legend_width);

    buf.set_string(start_x, area.y, "0", Style::default());
    let block_x = start_x + 2;
    for i in 0..steps {
        let ratio = (i as f64 + 1.0) / steps as f64;
        let color = count_to_color((ratio * max_count as f64) as usize, max_count);
        let style = Style::default().fg(Color::Black).bg(color);
        buf.set_string(block_x + i, area.y, " ", style);
    }
    buf.set_string(
        block_x + steps,
        area.y,
        format!(" {max_str}"),
        Style::default(),
    );
}

/// Render row labels (left) and column labels (bottom).
fn render_labels(data: &HeatmapData, layout: &HeatmapLayout, area_x: u16, buf: &mut Buffer) {
    for (i, label) in data.row_labels.iter().enumerate() {
        let y = layout.chart_y + (i as u16 * layout.cell_height);
        if y >= layout.bound_y {
            break;
        }
        let truncated: String = label
            .chars()
            .take(layout.row_label_width as usize)
            .collect();
        buf.set_string(area_x, y, &truncated, Style::default());
    }

    let col_label_y = layout.chart_y + layout.chart_height;
    if col_label_y < layout.bound_y {
        for (j, label) in data.col_labels.iter().enumerate() {
            let x = layout.chart_x + (j as u16 * layout.cell_width);
            if x >= layout.bound_x {
                break;
            }
            let truncated: String = label.chars().take(layout.cell_width as usize).collect();
            buf.set_string(x, col_label_y, &truncated, Style::default());
        }
    }
}

/// Render colored cells with count values.
fn render_cells(data: &HeatmapData, layout: &HeatmapLayout, buf: &mut Buffer) {
    for (i, row) in data.counts.iter().enumerate() {
        let y = layout.chart_y + (i as u16 * layout.cell_height);
        if y >= layout.bound_y {
            break;
        }
        for (j, &count) in row.iter().enumerate() {
            let x = layout.chart_x + (j as u16 * layout.cell_width);
            if x >= layout.bound_x {
                break;
            }
            let color = count_to_color(count, data.max_count);
            let text = format_count(count);
            let style = Style::default().fg(Color::Black).bg(color);
            let max_dy = layout.cell_height.min(layout.bound_y - y);
            for dy in 0..max_dy {
                for dx in 0..layout.cell_width.min(layout.bound_x - x) {
                    buf.set_string(x + dx, y + dy, " ", style);
                }
            }
            let text_y = y + layout.cell_height / 2;
            let text_x = x + (layout.cell_width.saturating_sub(text.len() as u16)) / 2;
            if text_x < layout.bound_x && text_y < layout.bound_y {
                buf.set_string(text_x, text_y, &text, style);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_to_color_zero() {
        assert_eq!(count_to_color(0, 10), Color::DarkGray);
    }

    #[test]
    fn test_count_to_color_max() {
        let color = count_to_color(10, 10);
        // At t=1.0: r=255, g=255, b=0 (yellow)
        assert_eq!(color, Color::Rgb(255, 255, 0));
    }

    #[test]
    fn test_format_count_zero() {
        assert_eq!(format_count(0), "·");
    }

    #[test]
    fn test_format_count_nonzero() {
        assert_eq!(format_count(5), "5");
        assert_eq!(format_count(42), "42");
    }

    #[test]
    fn test_heatmap_renders_without_panic() {
        let data = HeatmapData {
            title: Some("Test".to_string()),
            row_labels: vec!["A".to_string(), "B".to_string()],
            col_labels: vec!["X".to_string(), "Y".to_string()],
            counts: vec![vec![3, 1], vec![0, 5]],
            max_count: 5,
        };
        let area = Rect::new(0, 0, 40, 10);
        let mut buf = Buffer::empty(area);
        HeatmapChart::new(&data).render(area, &mut buf);

        // Check title rendered
        let first_line: String = (0..4)
            .map(|x| buf.cell((x, 0)).unwrap().symbol().to_string())
            .collect();
        assert_eq!(first_line.trim(), "Test");
    }

    #[test]
    fn test_heatmap_fills_entire_cell_height() {
        // 2 rows in a 10-row chart area (minus title=1, col_labels=1 → 8 rows)
        // cell_height = 8/2 = 4. Each cell should fill all 4 rows.
        let data = HeatmapData {
            title: Some("H".to_string()),
            row_labels: vec!["R1".to_string(), "R2".to_string()],
            col_labels: vec!["C1".to_string()],
            counts: vec![vec![5], vec![3]],
            max_count: 5,
        };
        let area = Rect::new(0, 0, 20, 10);
        let mut buf = Buffer::empty(area);
        HeatmapChart::new(&data).render(area, &mut buf);

        // Row label width = max("R1","R2") = 2, +1 = chart starts at x=3
        // Title at y=0, chart starts at y=1
        // cell_height = (10-1-1)/2 = 4
        // First cell: y=1..5
        let chart_x = 3_u16;
        let first_cell_color = count_to_color(5, 5);
        // Check that ALL rows of first cell have background color
        for dy in 0..4u16 {
            let cell = buf.cell((chart_x, 1 + dy)).unwrap();
            assert_eq!(
                cell.bg, first_cell_color,
                "Row {} of cell R1×C1 should be filled",
                dy
            );
        }
    }

    #[test]
    fn test_heatmap_renders_legend() {
        let data = HeatmapData {
            row_labels: vec!["A".into()],
            col_labels: vec!["X".into()],
            counts: vec![vec![5]],
            max_count: 5,
            title: Some("Test".into()),
        };
        let chart = HeatmapChart::new(&data);
        let area = Rect::new(0, 0, 40, 5);
        let mut buf = Buffer::empty(area);
        chart.render(area, &mut buf);
        // Legend should contain "0" and "5" (max_count)
        let content: String = (0..40)
            .map(|x| buf.cell((x, 0)).unwrap().symbol().to_string())
            .collect();
        assert!(
            content.contains('0') && content.contains('5'),
            "Legend should show scale 0..5, got: '{}'",
            content
        );
    }
}
