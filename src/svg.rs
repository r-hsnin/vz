//! SVG export: converts a ratatui Buffer into an SVG document (monospace text grid).

use ratatui::{
    buffer::Buffer,
    style::{Color, Style},
};
use std::fmt::Write;

/// Character width/height in the SVG coordinate space (pixels per cell).
const CELL_WIDTH: f64 = 8.0;
const CELL_HEIGHT: f64 = 16.0;
const FONT_SIZE: f64 = 14.0;

/// Convert a ratatui Buffer to an SVG string.
pub fn buffer_to_svg(buf: &Buffer) -> String {
    let area = buf.area;
    let width = area.width as f64 * CELL_WIDTH;
    let height = area.height as f64 * CELL_HEIGHT;

    let mut svg = String::with_capacity(4096);
    writeln!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="{width}" height="{height}">"#,
    )
    .unwrap();
    // Background
    writeln!(
        svg,
        r##"<rect width="100%" height="100%" fill="#1e1e1e"/>"##
    )
    .unwrap();
    // Font style
    writeln!(
        svg,
        r#"<style>text {{ font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace; font-size: {FONT_SIZE}px; white-space: pre; }}</style>"#,
    )
    .unwrap();

    // Render each row as a <text> element with colored <tspan>s
    for y in 0..area.height {
        let text_y = (y as f64 + 1.0) * CELL_HEIGHT - 3.0;
        write!(svg, r#"<text y="{text_y}">"#).unwrap();

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

        writeln!(svg, "</text>").unwrap();
    }

    writeln!(svg, "</svg>").unwrap();
    svg
}

fn write_tspan(svg: &mut String, x: f64, text: &str, style: Style) {
    let fg = style.fg.unwrap_or(Color::Gray);
    let color = color_to_hex(fg);
    let escaped = xml_escape(text);
    write!(svg, r#"<tspan x="{x}" fill="{color}">{escaped}</tspan>"#).unwrap();
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn color_to_hex(color: Color) -> &'static str {
    match color {
        Color::Black => "#000000",
        Color::Red => "#f44747",
        Color::Green => "#6a9955",
        Color::Yellow => "#dcdcaa",
        Color::Blue => "#569cd6",
        Color::Magenta => "#c586c0",
        Color::Cyan => "#4ec9b0",
        Color::Gray => "#cccccc",
        Color::DarkGray => "#808080",
        Color::LightRed => "#f14c4c",
        Color::LightGreen => "#b5cea8",
        Color::LightYellow => "#ffffcc",
        Color::LightBlue => "#9cdcfe",
        Color::LightMagenta => "#d7a0d7",
        Color::LightCyan => "#b5e8e0",
        Color::White => "#ffffff",
        _ => "#cccccc",
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
        let svg = buffer_to_svg(&buf);
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
        let svg = buffer_to_svg(&buf);
        assert!(svg.contains("#f44747"), "Red color expected");
        assert!(svg.contains("R"));
    }

    #[test]
    fn test_xml_escape() {
        assert_eq!(xml_escape("<test>&"), "&lt;test&gt;&amp;");
    }
}
