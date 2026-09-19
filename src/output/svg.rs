//! SVG export: converts a ratatui Buffer into an SVG document (monospace text grid).

use ratatui::{
    buffer::Buffer,
    style::{Color, Modifier, Style},
};
use std::fmt::Write;

/// Character width/height in the SVG coordinate space (pixels per cell).
const CELL_WIDTH: f64 = 8.0;
const CELL_HEIGHT: f64 = 16.0;
const FONT_SIZE: f64 = 14.0;

/// A real (resolution-independent) data point embedded in the SVG for
/// tooltips and programmatic use. Every bar / line vertex / scatter point /
/// histogram bin emits one `<circle class="vz-point" …>` with its label and
/// value; the text-grid layer below stays byte-identical for snapshots.
#[derive(Debug, Clone, PartialEq)]
pub struct DataPoint {
    /// Category / x label (e.g. city name, date string).
    pub label: String,
    /// Aggregated / plotted value.
    pub value: f64,
    /// Pixel-space position (same coordinate space as the text grid).
    pub x: f64,
    pub y: f64,
    /// Series name for multi-series line/scatter marks (`None` for bar/histogram/heatmap).
    pub series: Option<String>,
}

/// Render real vector data marks for one chart, in text-grid pixel space.
/// `width_cells`/`height_cells` are the buffer dimensions; the plot area is
/// the full area minus a left gutter (y-axis labels) and a bottom row
/// (x labels). Points carry labels+values so `--html` tooltips and
/// scrapers see data, not glyphs.
///
/// Layout contract (must match `buffer_to_svg` cell geometry):
/// - cell (0,0) is at pixel (0,0); each cell is CELL_W × CELL_H.
/// - bars grow upward from a baseline one row above the bottom edge.
/// - line/scatter/histogram points spread across the same plot rect.
pub fn data_marks_svg(
    chart_data: &crate::render::ChartData,
    width_cells: u16,
    height_cells: u16,
) -> String {
    let points = layout_points(chart_data, width_cells, height_cells);
    if points.is_empty() {
        return String::new();
    }
    let mut out = String::from("<g class=\"vz-data\" fill=\"#ffffff\" fill-opacity=\"0.0\">\n");
    for p in &points {
        let escaped = xml_escape(&format!(
            "{}: {}",
            p.label,
            crate::render::format_number(p.value)
        ));
        let series_attr = p
            .series
            .as_deref()
            .map(|s| format!(" data-series=\"{}\"", xml_escape(s)))
            .unwrap_or_default();
        let _ = writeln!(
            out,
            r#"<circle class="vz-point" cx="{:.1}" cy="{:.1}" r="7" data-label="{}" data-value="{}"{series_attr}><title>{}</title></circle>"#,
            p.x,
            p.y,
            xml_escape(&p.label),
            p.value,
            escaped,
        );
    }
    out.push_str("</g>\n");
    out
}

/// Compute pixel-space data positions for every plotted datum.
/// Bar: one point per category, centered in its slot, height ∝ value.
/// Line/Scatter: one point per vertex of every series, positioned on the
/// union axis span. Labels come from the shared `x_labels` by per-series
/// index, so overlaid series with missing points can mislabel tooltips
/// (see GOTCHAS).
/// Histogram: one point per bin at (bin_center, count).
/// Heatmap: one point per cell at (col, row) with the cell count.
fn layout_points(
    chart_data: &crate::render::ChartData,
    width_cells: u16,
    height_cells: u16,
) -> Vec<DataPoint> {
    use crate::render::ChartData;
    // Plot rect: leave a left gutter for y-axis labels and a bottom row
    // for x labels; title/border eat the top row.
    const GUTTER_CELLS: f64 = 6.0;
    let w = width_cells as f64 * CELL_WIDTH;
    let h = height_cells as f64 * CELL_HEIGHT;
    let plot_x0 = GUTTER_CELLS * CELL_WIDTH;
    let plot_x1 = w - CELL_WIDTH;
    let plot_y0 = CELL_HEIGHT * 1.5;
    let plot_y1 = h - CELL_HEIGHT * 1.5;
    if plot_x1 <= plot_x0 || plot_y1 <= plot_y0 {
        return vec![];
    }
    match chart_data {
        ChartData::Bar(d) => {
            let min = d
                .values
                .iter()
                .cloned()
                .fold(0.0_f64, f64::min)
                .min(-f64::EPSILON);
            let max = d
                .values
                .iter()
                .cloned()
                .fold(0.0_f64, f64::max)
                .max(f64::EPSILON);
            let n = d.labels.len().max(1) as f64;
            let span = (max - min).max(f64::EPSILON);
            d.labels
                .iter()
                .zip(d.values.iter())
                .enumerate()
                .map(|(i, (label, &v))| {
                    let cx = plot_x0 + (i as f64 + 0.5) * (plot_x1 - plot_x0) / n;
                    let frac = ((v - min) / span).clamp(0.0, 1.0);
                    let cy = plot_y1 - frac * (plot_y1 - plot_y0);
                    DataPoint {
                        label: label.clone(),
                        value: v,
                        x: cx,
                        y: cy,
                        series: None,
                    }
                })
                .collect()
        }
        ChartData::Line(c) | ChartData::Scatter(c) => {
            if c.series.iter().all(|s| s.data.is_empty()) {
                return vec![];
            }
            let (xmin, xmax) = c
                .series
                .iter()
                .flat_map(|s| s.data.iter().map(|(x, _)| *x))
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), x| {
                    (a.min(x), b.max(x))
                });
            let (ymin, ymax) = c
                .series
                .iter()
                .flat_map(|s| s.data.iter().map(|(_, y)| *y))
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), y| {
                    (a.min(y), b.max(y))
                });
            let xspan = (xmax - xmin).max(f64::EPSILON);
            let yspan = (ymax - ymin).max(f64::EPSILON);
            let labels = c.x_labels.clone().unwrap_or_default();
            c.series
                .iter()
                .flat_map(|series| {
                    series
                        .data
                        .iter()
                        .enumerate()
                        .map(|(i, &(x, y))| {
                            let cx = plot_x0 + (x - xmin) / xspan * (plot_x1 - plot_x0);
                            let cy = plot_y1 - (y - ymin) / yspan * (plot_y1 - plot_y0);
                            let label = labels.get(i).cloned().unwrap_or_else(|| format!("{x}"));
                            DataPoint {
                                label,
                                value: y,
                                x: cx,
                                y: cy,
                                series: Some(series.name.clone()),
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .collect()
        }
        ChartData::Histogram(d) => {
            let bins = crate::render::compute_bins(&d.values, d.bin_count);
            let max_c = bins
                .iter()
                .map(|(_, _, c)| *c as f64)
                .fold(0.0_f64, f64::max)
                .max(1.0);
            let n = bins.len().max(1) as f64;
            bins.iter()
                .enumerate()
                .map(|(i, (s, e, c))| {
                    let cx = plot_x0 + (i as f64 + 0.5) * (plot_x1 - plot_x0) / n;
                    let cy = plot_y1 - (*c as f64 / max_c) * (plot_y1 - plot_y0);
                    DataPoint {
                        label: format!(
                            "{}–{}",
                            crate::render::format_number(*s),
                            crate::render::format_number(*e)
                        ),
                        value: *c as f64,
                        x: cx,
                        y: cy,
                        series: None,
                    }
                })
                .collect()
        }
        ChartData::Heatmap(d) => {
            let rows = d.row_labels.len().max(1) as f64;
            let cols = d.col_labels.len().max(1) as f64;
            let mut pts = Vec::new();
            for (ri, rl) in d.row_labels.iter().enumerate() {
                for (ci, cl) in d.col_labels.iter().enumerate() {
                    let v = d
                        .counts
                        .get(ri)
                        .and_then(|r| r.get(ci))
                        .copied()
                        .unwrap_or(0) as f64;
                    pts.push(DataPoint {
                        label: format!("{rl} × {cl}"),
                        value: v,
                        x: plot_x0 + (ci as f64 + 0.5) * (plot_x1 - plot_x0) / cols,
                        y: plot_y0 + (ri as f64 + 0.5) * (plot_y1 - plot_y0) / rows,
                        series: None,
                    });
                }
            }
            pts
        }
    }
}

/// Convert a ratatui Buffer to an SVG string.
/// `bg_color` sets the background (default: dark theme #1e1e1e).
pub fn buffer_to_svg(buf: &Buffer, bg_color: &str) -> String {
    let area = buf.area;
    let width = area.width as f64 * CELL_WIDTH;
    let height = area.height as f64 * CELL_HEIGHT;

    let mut svg = String::with_capacity(4096);
    let _ = writeln!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="{width}" height="{height}">"#,
    );
    // Background
    let _ = writeln!(
        svg,
        r#"<rect width="100%" height="100%" fill="{bg_color}"/>"#,
    );
    // Font style
    let _ = writeln!(
        svg,
        r#"<style>text {{ font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace; font-size: {FONT_SIZE}px; white-space: pre; }}</style>"#,
    );

    // Render each row as a <text> element with colored <tspan>s
    for y in 0..area.height {
        let text_y = (y as f64 + 1.0) * CELL_HEIGHT - 3.0;
        let rect_y = y as f64 * CELL_HEIGHT;

        // Emit background rects for cells with non-default bg color
        let mut bg_start: Option<(u16, Color)> = None;
        for x in 0..area.width {
            let cell = &buf[(area.x + x, area.y + y)];
            let cell_bg = cell.style().bg;
            match (&mut bg_start, cell_bg) {
                (None, Some(color)) if color != Color::Reset => {
                    bg_start = Some((x, color));
                }
                (Some((start_x, prev_color)), Some(color))
                    if color != Color::Reset && color == *prev_color =>
                {
                    // continue span
                    let _ = (start_x, color);
                }
                (Some((start_x, prev_color)), _) => {
                    let rx = *start_x as f64 * CELL_WIDTH;
                    let rw = (x - *start_x) as f64 * CELL_WIDTH;
                    let fill = color_to_hex(*prev_color);
                    let _ = writeln!(
                        svg,
                        r#"<rect x="{rx}" y="{rect_y}" width="{rw}" height="{CELL_HEIGHT}" fill="{fill}"/>"#,
                    );
                    bg_start = cell_bg.filter(|c| *c != Color::Reset).map(|c| (x, c));
                }
                _ => {}
            }
        }
        // Flush remaining bg span
        if let Some((start_x, prev_color)) = bg_start {
            let rx = start_x as f64 * CELL_WIDTH;
            let rw = (area.width - start_x) as f64 * CELL_WIDTH;
            let fill = color_to_hex(prev_color);
            let _ = writeln!(
                svg,
                r#"<rect x="{rx}" y="{rect_y}" width="{rw}" height="{CELL_HEIGHT}" fill="{fill}"/>"#,
            );
        }

        let _ = write!(svg, r#"<text y="{text_y}">"#);

        let mut current_style = Style::default();
        let mut span_text = String::new();
        let mut span_x = 0.0;

        for x in 0..area.width {
            let cell = &buf[(area.x + x, area.y + y)];
            let style = cell.style();

            if style != current_style && !span_text.is_empty() {
                write_tspan(&mut svg, span_x, &span_text, current_style);
                span_text.clear();
                span_x = x as f64 * CELL_WIDTH;
            }
            if span_text.is_empty() {
                span_x = x as f64 * CELL_WIDTH;
                current_style = style;
            }
            span_text.push_str(cell.symbol());
        }

        // Flush last span
        if !span_text.trim_end().is_empty() {
            write_tspan(&mut svg, span_x, span_text.trim_end(), current_style);
        }

        let _ = writeln!(svg, "</text>");
    }

    let _ = writeln!(svg, "</svg>");
    svg
}

fn write_tspan(svg: &mut String, x: f64, text: &str, style: Style) {
    let fg = style.fg.unwrap_or(Color::Gray);
    let color = color_to_hex(fg);
    let escaped = xml_escape(text);
    let mut extra = String::new();
    if style.add_modifier.contains(Modifier::BOLD) {
        extra.push_str(r#" font-weight="bold""#);
    }
    if style.add_modifier.contains(Modifier::ITALIC) {
        extra.push_str(r#" font-style="italic""#);
    }
    let _ = write!(
        svg,
        r#"<tspan x="{x}" fill="{color}"{extra}>{escaped}</tspan>"#
    );
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn color_to_hex(color: Color) -> String {
    match color {
        Color::Black => "#000000".to_string(),
        Color::Red => "#f44747".to_string(),
        Color::Green => "#6a9955".to_string(),
        Color::Yellow => "#dcdcaa".to_string(),
        Color::Blue => "#569cd6".to_string(),
        Color::Magenta => "#c586c0".to_string(),
        Color::Cyan => "#4ec9b0".to_string(),
        Color::Gray => "#cccccc".to_string(),
        Color::DarkGray => "#808080".to_string(),
        Color::LightRed => "#f14c4c".to_string(),
        Color::LightGreen => "#b5cea8".to_string(),
        Color::LightYellow => "#ffffcc".to_string(),
        Color::LightBlue => "#9cdcfe".to_string(),
        Color::LightMagenta => "#d7a0d7".to_string(),
        Color::LightCyan => "#b5e8e0".to_string(),
        Color::White => "#ffffff".to_string(),
        Color::Rgb(r, g, b) => format!("#{:02x}{:02x}{:02x}", r, g, b),
        _ => "#cccccc".to_string(),
    }
}

/// Render a chart into an SVG string via an off-screen buffer.
/// The text-grid layer stays byte-identical to before; a `<g class="vz-data">`
/// overlay adds one real vector `<circle class="vz-point">` per datum with
/// `data-label`/`data-value` + `<title>` for tooltips and scraping.
/// (Breaking change: SVG output now contains a second layer — downstream
/// parsers that assumed "only text rows" must ignore the `vz-data` group.)
pub fn render_chart_svg(
    recommendation: &crate::chart::selector::ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
    opts: &crate::oneshot::RenderOptions<'_>,
) -> String {
    render_chart_svg_with_marks(recommendation, headers, rows, opts, true)
}

/// Same as [`render_chart_svg`] but with the data-marks overlay disabled.
/// Used by snapshot tests to pin the text-grid layer byte-for-byte.
pub fn render_chart_svg_text_only(
    recommendation: &crate::chart::selector::ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
    opts: &crate::oneshot::RenderOptions<'_>,
) -> String {
    render_chart_svg_with_marks(recommendation, headers, rows, opts, false)
}

fn render_chart_svg_with_marks(
    recommendation: &crate::chart::selector::ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
    opts: &crate::oneshot::RenderOptions<'_>,
    with_marks: bool,
) -> String {
    use ratatui::layout::Rect;

    let width = opts.width.unwrap_or_else(crate::oneshot::terminal_width);
    let chart_type = crate::oneshot::resolve_chart_type(recommendation, opts.chart_type_override);
    let height = opts.height.unwrap_or(crate::oneshot::DEFAULT_HEIGHT);

    let area = Rect::new(0, 0, width, height);
    let mut buf = Buffer::empty(area);
    let chart_data = crate::oneshot::build_chart_data_for_svg(
        chart_type,
        recommendation,
        headers,
        rows,
        opts,
        area,
    );
    crate::render::render_chart_data(&chart_data, area, &mut buf);

    let mut svg = buffer_to_svg(&buf, opts.theme.svg_background());
    if with_marks {
        let marks = data_marks_svg(&chart_data, width, height);
        if !marks.is_empty() {
            svg = svg.replacen("</svg>", &format!("{marks}</svg>"), 1);
        }
    }
    svg
}

/// Render the chart to SVG and print to stdout.
pub fn print_svg(
    recommendation: &crate::chart::selector::ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
    opts: &crate::oneshot::RenderOptions<'_>,
) -> anyhow::Result<()> {
    println!("{}", render_chart_svg(recommendation, headers, rows, opts));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    #[test]
    fn test_buffer_to_svg_basic() {
        let area = Rect::new(0, 0, 5, 2);
        let mut buf = Buffer::empty(area);
        buf[(0, 0)].set_char('H');
        buf[(1, 0)].set_char('i');
        let svg = buffer_to_svg(&buf, "#1e1e1e");
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("Hi"));
    }

    #[test]
    fn test_buffer_to_svg_colored() {
        let area = Rect::new(0, 0, 3, 1);
        let mut buf = Buffer::empty(area);
        buf[(0, 0)].set_char('R');
        buf[(0, 0)].set_style(Style::default().fg(Color::Red));
        let svg = buffer_to_svg(&buf, "#1e1e1e");
        assert!(svg.contains("#f44747"), "Red color expected");
        assert!(svg.contains("R"));
    }

    #[test]
    fn test_xml_escape() {
        assert_eq!(xml_escape("<test>&"), "&lt;test&gt;&amp;");
    }

    #[test]
    fn test_color_to_hex_rgb() {
        assert_eq!(color_to_hex(Color::Rgb(255, 128, 0)), "#ff8000");
        assert_eq!(color_to_hex(Color::Rgb(0, 0, 0)), "#000000");
        assert_eq!(color_to_hex(Color::Rgb(10, 200, 30)), "#0ac81e");
    }

    #[test]
    fn test_buffer_to_svg_with_background() {
        let area = Rect::new(0, 0, 3, 1);
        let mut buf = Buffer::empty(area);
        buf[(0, 0)].set_char('X');
        buf[(0, 0)].set_style(Style::default().fg(Color::Black).bg(Color::Rgb(255, 0, 0)));
        buf[(1, 0)].set_char('Y');
        buf[(1, 0)].set_style(Style::default().fg(Color::Black).bg(Color::Rgb(255, 0, 0)));
        let svg = buffer_to_svg(&buf, "#1e1e1e");
        // Should contain a rect with the red background
        assert!(svg.contains("#ff0000"), "Expected Rgb bg rect in SVG");
        assert!(svg.contains("<rect"), "Expected background rect element");
    }

    #[test]
    fn test_write_tspan_bold_italic() {
        let mut svg = String::new();
        let style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD | Modifier::ITALIC);
        write_tspan(&mut svg, 0.0, "Title", style);
        assert!(
            svg.contains(r#"font-weight="bold""#),
            "Expected bold attribute, got: {}",
            svg
        );
        assert!(
            svg.contains(r#"font-style="italic""#),
            "Expected italic attribute, got: {}",
            svg
        );
        assert!(svg.contains("Title"));
    }

    #[test]
    fn test_write_tspan_no_modifiers() {
        let mut svg = String::new();
        let style = Style::default().fg(Color::White);
        write_tspan(&mut svg, 8.0, "Plain", style);
        assert!(!svg.contains("font-weight"));
        assert!(!svg.contains("font-style"));
        assert!(svg.contains("Plain"));
    }

    #[test]
    fn test_render_chart_svg_produces_svg_document() {
        use crate::chart::selector::ChartType;
        use crate::test_helpers::make_recommendation;

        let rec = make_recommendation(ChartType::Bar, "city", Some("revenue"), None);
        let headers = vec!["city".to_string(), "revenue".to_string()];
        let rows = vec![
            vec!["Tokyo".to_string(), "100".to_string()],
            vec!["Osaka".to_string(), "200".to_string()],
        ];
        let opts = crate::oneshot::RenderOptions {
            chart_type_override: None,
            y_label_override: None,
            width: Some(60),
            height: Some(20),
            sort_order: None,
            extra_y_columns: vec![],
            limit: None,
            agg: crate::cli::AggFunction::Sum,
            title: None,
            labels: false,
            theme: crate::theme::Theme::default(),
            bins: None,
        };
        let svg = render_chart_svg(&rec, &headers, &rows, &opts);
        assert!(svg.starts_with("<svg"), "Should start with <svg tag");
        assert!(svg.contains("viewBox"), "Should have viewBox");
        assert!(svg.contains("</svg>"), "Should have closing tag");
    }

    #[test]
    fn test_data_marks_bar_carries_labels_and_values() {
        use crate::render::{BarChartData, ChartData};
        let data = ChartData::Bar(BarChartData {
            title: None,
            labels: vec!["Tokyo".to_string(), "Osaka".to_string()],
            values: vec![4200.0, 3300.0],
            y_label: "revenue".to_string(),
            show_labels: false,
            series_colors: vec![],
            axis_color: None,
        });
        let marks = data_marks_svg(&data, 80, 24);
        assert_eq!(
            marks.matches("vz-point").count(),
            2,
            "one mark per bar: {marks}"
        );
        assert!(
            marks.contains("data-label=\"Tokyo\""),
            "label missing: {marks}"
        );
        assert!(
            marks.contains("data-value=\"4200"),
            "value missing: {marks}"
        );
        assert!(
            marks.contains("<title>Tokyo:"),
            "tooltip title missing: {marks}"
        );
    }

    #[test]
    fn test_data_marks_line_vertex_per_point() {
        use crate::render::{Axis, ChartConfig, ChartData, Series};
        let data = ChartData::Line(ChartConfig {
            title: None,
            x_axis: Axis {
                label: "d".into(),
                min: 0.0,
                max: 2.0,
            },
            y_axis: Axis {
                label: "v".into(),
                min: 0.0,
                max: 30.0,
            },
            series: vec![Series {
                name: "v".into(),
                data: vec![(0.0, 10.0), (1.0, 20.0), (2.0, 30.0)],
            }],
            x_labels: Some(vec!["a".into(), "b".into(), "c".into()]),
            series_colors: vec![],
            axis_color: None,
            label_color: None,
        });
        let marks = data_marks_svg(&data, 80, 24);
        assert_eq!(
            marks.matches("vz-point").count(),
            3,
            "one mark per vertex: {marks}"
        );
    }

    #[test]
    fn test_data_marks_multi_series_cover_all_points() {
        use crate::render::{Axis, ChartConfig, ChartData, Series};
        let data = ChartData::Line(ChartConfig {
            title: None,
            x_axis: Axis {
                label: "d".into(),
                min: 0.0,
                max: 2.0,
            },
            y_axis: Axis {
                label: "v".into(),
                min: 0.0,
                max: 30.0,
            },
            series: vec![
                Series {
                    name: "a".into(),
                    data: vec![(0.0, 10.0), (1.0, 20.0)],
                },
                Series {
                    name: "b".into(),
                    data: vec![(0.0, 5.0), (1.0, 25.0)],
                },
            ],
            x_labels: Some(vec!["x0".into(), "x1".into()]),
            series_colors: vec![],
            axis_color: None,
            label_color: None,
        });
        let marks = data_marks_svg(&data, 80, 24);
        assert_eq!(
            marks.matches("vz-point").count(),
            4,
            "every series vertex needs a mark: {marks}"
        );
        assert!(
            marks.contains("data-series=\"b\""),
            "series name missing: {marks}"
        );
    }

    #[test]
    fn test_data_marks_bar_negative_values_stay_in_plot() {
        // Bar layout spans min..max so negatives sit below positives.
        use crate::render::{BarChartData, ChartData};
        let data = ChartData::Bar(BarChartData {
            title: None,
            labels: vec!["loss".to_string(), "gain".to_string()],
            values: vec![-50.0, 100.0],
            y_label: "pnl".to_string(),
            show_labels: false,
            series_colors: vec![],
            axis_color: None,
        });
        let pts = layout_points(&data, 80, 24);
        assert_eq!(pts.len(), 2);
        assert!(
            pts[0].y > pts[1].y,
            "negative bar must sit below positive bar (loss {:?} vs gain {:?})",
            pts[0],
            pts[1]
        );
    }

    #[test]
    fn test_data_marks_empty_chart_emits_nothing() {
        use crate::render::{BarChartData, ChartData};
        let data = ChartData::Bar(BarChartData {
            title: None,
            labels: vec![],
            values: vec![],
            y_label: "y".into(),
            show_labels: false,
            series_colors: vec![],
            axis_color: None,
        });
        assert!(data_marks_svg(&data, 80, 24).is_empty());
    }

    #[test]
    fn test_render_chart_svg_embeds_data_overlay() {
        use crate::chart::selector::ChartType;
        use crate::test_helpers::make_recommendation;
        let rec = make_recommendation(ChartType::Bar, "city", Some("revenue"), None);
        let headers = vec!["city".to_string(), "revenue".to_string()];
        let rows = vec![
            vec!["Tokyo".to_string(), "100".to_string()],
            vec!["Osaka".to_string(), "200".to_string()],
        ];
        let opts = crate::oneshot::RenderOptions {
            chart_type_override: None,
            y_label_override: None,
            width: Some(60),
            height: Some(20),
            sort_order: None,
            extra_y_columns: vec![],
            limit: None,
            agg: crate::cli::AggFunction::Sum,
            title: None,
            labels: false,
            theme: crate::theme::Theme::default(),
            bins: None,
        };
        let svg = render_chart_svg(&rec, &headers, &rows, &opts);
        assert!(svg.contains("vz-point"), "expected data overlay in SVG");
        assert!(svg.contains("data-label=\"Tokyo\""), "expected Tokyo mark");
    }
}
