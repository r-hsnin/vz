//! Color theme definitions for chart rendering.

use ratatui::style::Color;

/// A color theme for chart rendering.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    /// Colors used for data series (cycled if more series than colors).
    pub series_colors: Vec<Color>,
    /// Color for axis lines and borders.
    pub axis_color: Color,
    /// Color for axis labels and tick text.
    pub label_color: Color,
    /// Color for chart title.
    pub title_color: Color,
    /// Color for grid lines (if any).
    pub grid_color: Color,
}

impl Theme {
    /// Dark theme — optimized for dark terminal backgrounds (default).
    pub fn dark() -> Self {
        Self {
            series_colors: vec![
                Color::Cyan,
                Color::Yellow,
                Color::Green,
                Color::Magenta,
                Color::Red,
                Color::Blue,
            ],
            axis_color: Color::DarkGray,
            label_color: Color::Gray,
            title_color: Color::White,
            grid_color: Color::DarkGray,
        }
    }

    /// Light theme — optimized for light/white terminal backgrounds.
    pub fn light() -> Self {
        Self {
            series_colors: vec![
                Color::Blue,
                Color::Red,
                Color::Green,
                Color::Magenta,
                Color::Cyan,
                Color::Yellow,
            ],
            axis_color: Color::Gray,
            label_color: Color::DarkGray,
            title_color: Color::Black,
            grid_color: Color::Gray,
        }
    }

    /// High-contrast theme — maximum visibility, colorblind-friendly ordering.
    pub fn high_contrast() -> Self {
        Self {
            series_colors: vec![
                Color::White,
                Color::Yellow,
                Color::Cyan,
                Color::Green,
                Color::Magenta,
                Color::Red,
            ],
            axis_color: Color::White,
            label_color: Color::White,
            title_color: Color::White,
            grid_color: Color::White,
        }
    }

    /// Get the series color at a given index (wraps around).
    pub fn series_color(&self, index: usize) -> Color {
        self.series_colors[index % self.series_colors.len()]
    }

    /// Get the SVG background color as a hex string.
    pub fn svg_background(&self) -> &'static str {
        match self.title_color {
            Color::Black => "#ffffff", // light theme
            Color::White => "#1e1e1e", // dark theme
            _ => "#0a0a0a",            // high-contrast
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dark_theme_has_six_colors() {
        assert_eq!(Theme::dark().series_colors.len(), 6);
    }

    #[test]
    fn test_light_theme_has_six_colors() {
        assert_eq!(Theme::light().series_colors.len(), 6);
    }

    #[test]
    fn test_high_contrast_theme_has_six_colors() {
        assert_eq!(Theme::high_contrast().series_colors.len(), 6);
    }

    #[test]
    fn test_series_color_wraps() {
        let theme = Theme::dark();
        assert_eq!(theme.series_color(0), theme.series_color(6));
        assert_eq!(theme.series_color(1), theme.series_color(7));
    }

    #[test]
    fn test_default_is_dark() {
        assert_eq!(Theme::default(), Theme::dark());
    }

    #[test]
    fn test_themes_have_different_axis_colors() {
        let dark = Theme::dark();
        let light = Theme::light();
        assert_ne!(dark.axis_color, light.axis_color);
        assert_ne!(dark.title_color, light.title_color);
    }
}
