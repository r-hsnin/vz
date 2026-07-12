//! ANSI terminal output: Buffer → colored text.

use std::io::{IsTerminal, Write};

use ratatui::{
    buffer::Buffer,
    style::{Color, Modifier, Style},
};

/// Determine whether to output ANSI color codes (for stdout).
/// Respects NO_COLOR env var (https://no-color.org/) and TTY detection.
pub fn should_colorize() -> bool {
    // NO_COLOR takes precedence (any non-empty value disables color)
    if std::env::var("NO_COLOR").is_ok_and(|v| !v.is_empty()) {
        return false;
    }
    // FORCE_COLOR forces color on
    if std::env::var("FORCE_COLOR").is_ok_and(|v| !v.is_empty()) {
        return true;
    }
    // Default: color only if stdout is a TTY
    std::io::stdout().is_terminal()
}

/// Determine whether to output ANSI color codes to stderr.
/// Used by the summary line which is always printed to stderr.
pub fn should_colorize_stderr() -> bool {
    if std::env::var("NO_COLOR").is_ok_and(|v| !v.is_empty()) {
        return false;
    }
    if std::env::var("FORCE_COLOR").is_ok_and(|v| !v.is_empty()) {
        return true;
    }
    std::io::stderr().is_terminal()
}

/// Convert a ratatui Buffer to ANSI-colored text and write to the given writer.
pub fn print_buffer<W: Write>(buf: &Buffer, writer: &mut W) -> anyhow::Result<()> {
    let colorize = should_colorize();
    let area = buf.area;

    for y in area.y..area.y + area.height {
        let mut last_style = Style::default();
        let mut line = String::new();

        for x in area.x..area.x + area.width {
            let cell = &buf[(x, y)];
            let style = cell.style();

            if colorize && style != last_style {
                // Reset previous style
                if last_style != Style::default() {
                    line.push_str("\x1b[0m");
                }
                // Apply new style
                let ansi = style_to_ansi(style);
                if !ansi.is_empty() {
                    line.push_str(&ansi);
                }
                last_style = style;
            }

            line.push_str(cell.symbol());
        }

        // Reset at end of line
        if colorize && last_style != Style::default() {
            line.push_str("\x1b[0m");
        }

        // Trim trailing spaces (but preserve ANSI resets)
        let trimmed = line.trim_end();
        writeln!(writer, "{}", trimmed)?;
    }

    Ok(())
}

/// Convert a ratatui Style to ANSI escape sequence.
pub fn style_to_ansi(style: Style) -> String {
    let mut codes: Vec<String> = Vec::new();

    if let Some(fg) = style.fg
        && let Some(code) = color_to_ansi(fg, false)
    {
        codes.push(code);
    }

    if let Some(bg) = style.bg
        && let Some(code) = color_to_ansi(bg, true)
    {
        codes.push(code);
    }

    if style.add_modifier.contains(Modifier::BOLD) {
        codes.push("1".to_string());
    }
    if style.add_modifier.contains(Modifier::DIM) {
        codes.push("2".to_string());
    }
    if style.add_modifier.contains(Modifier::ITALIC) {
        codes.push("3".to_string());
    }
    if style.add_modifier.contains(Modifier::UNDERLINED) {
        codes.push("4".to_string());
    }

    if codes.is_empty() {
        String::new()
    } else {
        format!("\x1b[{}m", codes.join(";"))
    }
}

/// Map ratatui Color to ANSI escape code. `is_bg` selects background (offset +10).
fn color_to_ansi(color: Color, is_bg: bool) -> Option<String> {
    let offset: u8 = if is_bg { 10 } else { 0 };
    match color {
        Color::Black => Some(format!("{}", 30 + offset)),
        Color::Red => Some(format!("{}", 31 + offset)),
        Color::Green => Some(format!("{}", 32 + offset)),
        Color::Yellow => Some(format!("{}", 33 + offset)),
        Color::Blue => Some(format!("{}", 34 + offset)),
        Color::Magenta => Some(format!("{}", 35 + offset)),
        Color::Cyan => Some(format!("{}", 36 + offset)),
        Color::Gray => Some(format!("{}", 37 + offset)),
        Color::DarkGray => Some(format!("{}", 90 + offset)),
        Color::LightRed => Some(format!("{}", 91 + offset)),
        Color::LightGreen => Some(format!("{}", 92 + offset)),
        Color::LightYellow => Some(format!("{}", 93 + offset)),
        Color::LightBlue => Some(format!("{}", 94 + offset)),
        Color::LightMagenta => Some(format!("{}", 95 + offset)),
        Color::LightCyan => Some(format!("{}", 96 + offset)),
        Color::White => Some(format!("{}", 97 + offset)),
        Color::Indexed(idx) => {
            let prefix = if is_bg { 48 } else { 38 };
            Some(format!("{};5;{}", prefix, idx))
        }
        Color::Rgb(r, g, b) => {
            let prefix = if is_bg { 48 } else { 38 };
            Some(format!("{};2;{};{};{}", prefix, r, g, b))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    #[test]
    fn test_print_buffer_basic() {
        let area = Rect::new(0, 0, 10, 3);
        let mut buf = Buffer::empty(area);

        buf[(0, 0)].set_char('H');
        buf[(1, 0)].set_char('i');

        let mut output = Vec::new();
        print_buffer(&buf, &mut output).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("Hi"));
    }

    #[test]
    fn test_print_buffer_with_color() {
        let area = Rect::new(0, 0, 5, 1);
        let mut buf = Buffer::empty(area);

        buf[(0, 0)].set_char('R');
        buf[(0, 0)].set_style(Style::default().fg(Color::Red));

        let mut output = Vec::new();
        print_buffer(&buf, &mut output).unwrap();
        let text = String::from_utf8(output).unwrap();
        // Should contain ANSI red color code
        assert!(text.contains("\x1b[31m"));
        assert!(text.contains("R"));
        assert!(text.contains("\x1b[0m"));
    }

    #[test]
    fn test_style_to_ansi_empty() {
        let ansi = style_to_ansi(Style::default());
        assert_eq!(ansi, "");
    }

    #[test]
    fn test_style_to_ansi_fg_color() {
        let style = Style::default().fg(Color::Cyan);
        let ansi = style_to_ansi(style);
        assert_eq!(ansi, "\x1b[36m");
    }

    #[test]
    fn test_style_to_ansi_bold() {
        let style = Style::default().add_modifier(Modifier::BOLD);
        let ansi = style_to_ansi(style);
        assert_eq!(ansi, "\x1b[1m");
    }

    #[test]
    fn test_style_to_ansi_combined() {
        let style = Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD);
        let ansi = style_to_ansi(style);
        assert!(ansi.contains("32"));
        assert!(ansi.contains("1"));
    }

    #[test]
    fn test_color_to_ansi_fg() {
        assert_eq!(color_to_ansi(Color::Red, false), Some("31".to_string()));
        assert_eq!(color_to_ansi(Color::Green, false), Some("32".to_string()));
    }

    #[test]
    fn test_color_to_ansi_bg() {
        assert_eq!(color_to_ansi(Color::Red, true), Some("41".to_string()));
        assert_eq!(color_to_ansi(Color::Cyan, true), Some("46".to_string()));
    }

    #[test]
    fn test_color_to_ansi_indexed() {
        assert_eq!(
            color_to_ansi(Color::Indexed(42), false),
            Some("38;5;42".to_string())
        );
    }

    #[test]
    fn test_color_to_ansi_rgb() {
        assert_eq!(
            color_to_ansi(Color::Rgb(255, 128, 0), false),
            Some("38;2;255;128;0".to_string())
        );
    }
}
