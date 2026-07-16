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
}
