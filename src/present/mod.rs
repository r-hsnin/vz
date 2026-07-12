use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};
use std::path::Path;

use crate::chart::selector::ChartType;

/// A single slide in a presentation.
#[derive(Debug, Clone, PartialEq)]
pub struct Slide {
    pub title: Option<String>,
    pub content: Vec<SlideElement>,
}

/// Elements that can appear on a slide.
#[derive(Debug, Clone, PartialEq)]
pub enum SlideElement {
    /// Plain text paragraph.
    Text(String),
    /// Bullet point list.
    Bullets(Vec<String>),
    /// Chart block with configuration.
    Chart(ChartBlock),
    /// Fenced code block with optional language.
    Code {
        language: Option<String>,
        content: String,
    },
    /// Sub-heading (## or ###) within a slide.
    Heading { level: u8, text: String },
    /// Numbered/ordered list.
    OrderedList(Vec<String>),
}

/// Configuration for a chart embedded in a slide.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartBlock {
    pub source: String,
    pub chart_type: Option<ChartType>,
    pub x_col: Option<String>,
    pub y_col: Option<String>,
    pub color_col: Option<String>,
    pub title: Option<String>,
    /// Optional filter expressions (same syntax as `--where`).
    pub filter: Vec<String>,
}

/// A parsed presentation.
#[derive(Debug, Clone, PartialEq)]
pub struct Presentation {
    pub slides: Vec<Slide>,
}

/// Application state for Present mode.
pub struct PresentApp {
    pub presentation: Presentation,
    pub current_slide: usize,
    pub should_quit: bool,
    pub base_dir: std::path::PathBuf,
    /// Buffer for digit input (jump-to-slide).
    pub input_buffer: String,
    /// Color theme for chart rendering.
    pub theme: crate::theme::Theme,
}

impl PresentApp {
    pub fn new(
        presentation: Presentation,
        base_dir: std::path::PathBuf,
        theme: crate::theme::Theme,
    ) -> Self {
        Self {
            presentation,
            current_slide: 0,
            should_quit: false,
            base_dir,
            input_buffer: String::new(),
            theme,
        }
    }

    pub fn handle_key(&mut self, key: KeyCode) {
        // If we have pending digit input, handle it specially
        if !self.input_buffer.is_empty() {
            match key {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    self.input_buffer.push(c);
                    return;
                }
                KeyCode::Enter => {
                    self.execute_jump();
                    return;
                }
                KeyCode::Esc => {
                    self.input_buffer.clear();
                    return;
                }
                _ => {
                    self.input_buffer.clear();
                    // Fall through to normal handling
                }
            }
        }

        match key {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Char(' ') => {
                if self.current_slide < self.presentation.slides.len().saturating_sub(1) {
                    self.current_slide += 1;
                }
            }
            KeyCode::Enter => {
                if self.current_slide < self.presentation.slides.len().saturating_sub(1) {
                    self.current_slide += 1;
                }
            }
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Backspace => {
                if self.current_slide > 0 {
                    self.current_slide -= 1;
                }
            }
            KeyCode::Home | KeyCode::Char('g') => self.current_slide = 0,
            KeyCode::Char('G') | KeyCode::End => {
                self.current_slide = self.presentation.slides.len().saturating_sub(1);
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                self.input_buffer.push(c);
            }
            _ => {}
        }
    }

    /// Execute jump from input buffer (1-based slide number).
    fn execute_jump(&mut self) {
        if let Ok(n) = self.input_buffer.parse::<usize>() {
            let max = self.presentation.slides.len().saturating_sub(1);
            // Input is 1-based, convert to 0-based. 0 → first slide.
            self.current_slide = if n == 0 { 0 } else { (n - 1).min(max) };
        }
        self.input_buffer.clear();
    }

    pub fn current_slide(&self) -> Option<&Slide> {
        self.presentation.slides.get(self.current_slide)
    }

    pub fn slide_indicator(&self) -> String {
        if self.input_buffer.is_empty() {
            format!(
                "{}/{}",
                self.current_slide + 1,
                self.presentation.slides.len()
            )
        } else {
            format!(
                "→{} ({}/{})",
                self.input_buffer,
                self.current_slide + 1,
                self.presentation.slides.len()
            )
        }
    }
}

/// Parse a markdown file into a Presentation.
mod parser;

pub use parser::parse_presentation;

/// Parse inline markdown formatting (**bold** and *italic*) into styled spans.
pub fn parse_inline_spans(text: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        // Try bold first (**)
        if let Some(start) = remaining.find("**")
            && let Some(end) = remaining[start + 2..].find("**")
        {
            if start > 0 {
                spans.push(Span::raw(remaining[..start].to_string()));
            }
            let bold_text = &remaining[start + 2..start + 2 + end];
            spans.push(Span::styled(
                bold_text.to_string(),
                Style::default().add_modifier(Modifier::BOLD),
            ));
            remaining = &remaining[start + 2 + end + 2..];
            continue;
        }
        // Try italic (*)
        if let Some(start) = remaining.find('*')
            && let Some(end) = remaining[start + 1..].find('*')
        {
            if start > 0 {
                spans.push(Span::raw(remaining[..start].to_string()));
            }
            let italic_text = &remaining[start + 1..start + 1 + end];
            spans.push(Span::styled(
                italic_text.to_string(),
                Style::default().add_modifier(Modifier::ITALIC),
            ));
            remaining = &remaining[start + 1 + end + 1..];
            continue;
        }
        // Try code (`)
        if let Some(start) = remaining.find('`')
            && let Some(end) = remaining[start + 1..].find('`')
        {
            if start > 0 {
                spans.push(Span::raw(remaining[..start].to_string()));
            }
            let code_text = &remaining[start + 1..start + 1 + end];
            spans.push(Span::styled(
                code_text.to_string(),
                Style::default().fg(Color::Yellow),
            ));
            remaining = &remaining[start + 1 + end + 1..];
            continue;
        }
        // No more formatting
        spans.push(Span::raw(remaining.to_string()));
        break;
    }

    if spans.is_empty() {
        spans.push(Span::raw(text.to_string()));
    }
    spans
}

/// Run the Present mode TUI.
pub fn run_present(path: &Path, theme: crate::theme::Theme) -> Result<()> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read presentation file: {:?}", path))?;
    let presentation = parse_presentation(&content);

    if presentation.slides.is_empty() {
        anyhow::bail!("No slides found in {:?}", path);
    }

    // Resolve the base directory for relative chart source paths
    let base_dir = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    let mut terminal = ratatui::init();
    let mut app = PresentApp::new(presentation, base_dir, theme);

    loop {
        terminal.draw(|frame| render::draw_slide(frame, &app))?;

        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            app.handle_key(key.code);
        }

        if app.should_quit {
            break;
        }
    }

    ratatui::restore();
    Ok(())
}

mod chart_loader;
mod render;

/// Load chart data for a presentation chart block (delegated to chart_loader module).
pub(crate) fn load_chart_data(
    block: &ChartBlock,
    base_dir: &Path,
    theme: &crate::theme::Theme,
) -> Result<crate::render::ChartData> {
    chart_loader::load_chart_data(block, base_dir, theme)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_presentation() {
        let content = r#"# Hello World

Welcome to vz.

---

# Slide 2

- Point one
- Point two
- Point three
"#;
        let pres = parse_presentation(content);
        assert_eq!(pres.slides.len(), 2);
        assert_eq!(pres.slides[0].title, Some("Hello World".to_string()));
        assert_eq!(pres.slides[1].title, Some("Slide 2".to_string()));
    }

    #[test]
    fn test_parse_bullets() {
        let content = "# Lists\n\n- Alpha\n- Beta\n- Gamma\n";
        let pres = parse_presentation(content);
        assert_eq!(pres.slides.len(), 1);
        match &pres.slides[0].content[0] {
            SlideElement::Bullets(items) => {
                assert_eq!(items.len(), 3);
                assert_eq!(items[0], "Alpha");
            }
            _ => panic!("Expected Bullets"),
        }
    }

    #[test]
    fn test_parse_chart_block() {
        let content = r#"# Revenue

```chart
source: sales.csv
x: month
y: revenue
type: bar
title: Monthly Revenue
```
"#;
        let pres = parse_presentation(content);
        assert_eq!(pres.slides.len(), 1);
        match &pres.slides[0].content[0] {
            SlideElement::Chart(chart) => {
                assert_eq!(chart.source, "sales.csv");
                assert_eq!(chart.x_col, Some("month".to_string()));
                assert_eq!(chart.y_col, Some("revenue".to_string()));
                assert_eq!(chart.chart_type, Some(ChartType::Bar));
                assert_eq!(chart.title, Some("Monthly Revenue".to_string()));
            }
            _ => panic!("Expected Chart"),
        }
    }

    #[test]
    fn test_parse_chart_block_with_where() {
        let content = r#"# Filtered

```chart
source: sales.csv
x: date
y: revenue
where: city=Tokyo
where: revenue>500
```
"#;
        let pres = parse_presentation(content);
        match &pres.slides[0].content[0] {
            SlideElement::Chart(chart) => {
                assert_eq!(chart.filter, vec!["city=Tokyo", "revenue>500"]);
            }
            _ => panic!("Expected Chart"),
        }
    }

    #[test]
    fn test_parse_chart_block_minimal() {
        let content = "# Chart\n\n```chart\nsource: data.csv\n```\n";
        let pres = parse_presentation(content);
        match &pres.slides[0].content[0] {
            SlideElement::Chart(chart) => {
                assert_eq!(chart.source, "data.csv");
                assert_eq!(chart.chart_type, None);
                assert_eq!(chart.x_col, None);
                assert_eq!(chart.y_col, None);
            }
            _ => panic!("Expected Chart"),
        }
    }

    #[test]
    fn test_parse_separator_based() {
        let content = "---\nFirst slide\n---\nSecond slide\n---\nThird slide\n";
        let pres = parse_presentation(content);
        assert_eq!(pres.slides.len(), 3);
    }

    #[test]
    fn test_present_app_navigation() {
        let pres = Presentation {
            slides: vec![
                Slide {
                    title: Some("1".into()),
                    content: vec![],
                },
                Slide {
                    title: Some("2".into()),
                    content: vec![],
                },
                Slide {
                    title: Some("3".into()),
                    content: vec![],
                },
            ],
        };
        let mut app = PresentApp::new(pres, std::path::PathBuf::new(), crate::theme::Theme::dark());
        assert_eq!(app.current_slide, 0);

        app.handle_key(KeyCode::Right);
        assert_eq!(app.current_slide, 1);

        app.handle_key(KeyCode::Right);
        assert_eq!(app.current_slide, 2);

        // Can't go past the end
        app.handle_key(KeyCode::Right);
        assert_eq!(app.current_slide, 2);

        app.handle_key(KeyCode::Left);
        assert_eq!(app.current_slide, 1);

        // Jump to start
        app.handle_key(KeyCode::Char('g'));
        assert_eq!(app.current_slide, 0);

        // Jump to end
        app.handle_key(KeyCode::Char('G'));
        assert_eq!(app.current_slide, 2);
    }

    #[test]
    fn test_present_app_quit() {
        let pres = Presentation {
            slides: vec![Slide {
                title: None,
                content: vec![],
            }],
        };
        let mut app = PresentApp::new(pres, std::path::PathBuf::new(), crate::theme::Theme::dark());
        assert!(!app.should_quit);
        app.handle_key(KeyCode::Char('q'));
        assert!(app.should_quit);
    }

    #[test]
    fn test_slide_indicator() {
        let pres = Presentation {
            slides: vec![
                Slide {
                    title: None,
                    content: vec![],
                },
                Slide {
                    title: None,
                    content: vec![],
                },
                Slide {
                    title: None,
                    content: vec![],
                },
            ],
        };
        let mut app = PresentApp::new(pres, std::path::PathBuf::new(), crate::theme::Theme::dark());
        assert_eq!(app.slide_indicator(), "1/3");
        app.handle_key(KeyCode::Right);
        assert_eq!(app.slide_indicator(), "2/3");
    }

    #[test]
    fn test_parse_mixed_content() {
        let content = r#"# Overview

Here is some text.

- Bullet A
- Bullet B

```chart
source: metrics.csv
type: line
```

More text after chart.
"#;
        let pres = parse_presentation(content);
        assert_eq!(pres.slides.len(), 1);
        let elements = &pres.slides[0].content;
        assert_eq!(elements.len(), 4); // text, bullets, chart, text
        assert!(matches!(&elements[0], SlideElement::Text(_)));
        assert!(matches!(&elements[1], SlideElement::Bullets(_)));
        assert!(matches!(&elements[2], SlideElement::Chart(_)));
        assert!(matches!(&elements[3], SlideElement::Text(_)));
    }

    #[test]
    fn test_empty_presentation() {
        let pres = parse_presentation("");
        assert_eq!(pres.slides.len(), 0);
    }

    #[test]
    fn test_load_chart_data_relative_to_base_dir() {
        // Create the chart block as it would appear in demo.md
        let block = ChartBlock {
            source: "sales.csv".to_string(),
            chart_type: Some(ChartType::Line),
            x_col: Some("date".to_string()),
            y_col: Some("revenue".to_string()),
            color_col: None,
            title: Some("Revenue Trend".to_string()),
            filter: vec![],
        };

        // Use the fixtures directory as base_dir (where demo.md lives)
        let base_dir = std::path::Path::new("fixtures");
        let result = super::load_chart_data(&block, base_dir, &crate::theme::Theme::dark());
        assert!(
            result.is_ok(),
            "load_chart_data should resolve sales.csv relative to fixtures/: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_load_chart_data_with_filter() {
        let block = ChartBlock {
            source: "sales.csv".to_string(),
            chart_type: Some(ChartType::Bar),
            x_col: Some("city".to_string()),
            y_col: Some("revenue".to_string()),
            color_col: None,
            title: None,
            filter: vec!["city=Tokyo".to_string()],
        };

        let base_dir = std::path::Path::new("fixtures");
        let result = super::load_chart_data(&block, base_dir, &crate::theme::Theme::dark());
        assert!(
            result.is_ok(),
            "load_chart_data with where filter should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_load_chart_data_nonexistent_source() {
        let block = ChartBlock {
            source: "nonexistent.csv".to_string(),
            chart_type: None,
            x_col: None,
            y_col: None,
            color_col: None,
            title: None,
            filter: vec![],
        };

        let base_dir = std::path::Path::new("fixtures");
        let result = super::load_chart_data(&block, base_dir, &crate::theme::Theme::dark());
        assert!(result.is_err());
    }

    #[test]
    fn test_load_chart_data_with_color_column() {
        let block = ChartBlock {
            source: "sales.csv".to_string(),
            chart_type: Some(ChartType::Line),
            x_col: Some("date".to_string()),
            y_col: Some("revenue".to_string()),
            color_col: Some("city".to_string()),
            title: Some("Multi-Series".to_string()),
            filter: vec![],
        };

        let base_dir = std::path::Path::new("fixtures");
        let result = super::load_chart_data(&block, base_dir, &crate::theme::Theme::dark());
        assert!(
            result.is_ok(),
            "multi-series should work: {:?}",
            result.err()
        );

        if let Ok(crate::render::ChartData::Line(config)) = result {
            // Should have multiple series (one per city)
            assert!(
                config.series.len() > 1,
                "Expected multi-series, got {}",
                config.series.len()
            );
            // Check series names
            let names: Vec<&str> = config.series.iter().map(|s| s.name.as_str()).collect();
            assert!(names.contains(&"Tokyo"));
            assert!(names.contains(&"Osaka"));
        } else {
            panic!("Expected Line chart data");
        }
    }

    #[test]
    fn test_parse_chart_block_with_color() {
        let lines = vec![
            "source: data.csv".to_string(),
            "x: month".to_string(),
            "y: revenue".to_string(),
            "color: region".to_string(),
            "type: line".to_string(),
            "title: By Region".to_string(),
        ];
        let block = super::parser::parse_chart_block(&lines);
        assert_eq!(block.color_col, Some("region".to_string()));
        assert_eq!(block.title, Some("By Region".to_string()));
    }

    #[test]
    fn test_load_chart_data_json_source() {
        let block = ChartBlock {
            source: "scores.json".to_string(),
            chart_type: Some(ChartType::Bar),
            x_col: Some("name".to_string()),
            y_col: Some("score".to_string()),
            color_col: None,
            title: Some("Scores".to_string()),
            filter: vec![],
        };

        let base_dir = std::path::Path::new("fixtures");
        let result = super::load_chart_data(&block, base_dir, &crate::theme::Theme::dark());
        assert!(
            result.is_ok(),
            "JSON source should work via loader: {:?}",
            result.err()
        );

        if let Ok(crate::render::ChartData::Bar(data)) = result {
            assert_eq!(data.labels, vec!["Alice", "Bob", "Charlie"]);
            assert_eq!(data.values, vec![85.0, 92.0, 78.0]);
        } else {
            panic!("Expected Bar chart data from JSON source");
        }
    }

    #[test]
    fn test_load_chart_data_infers_type_when_not_specified() {
        // departments.csv has 2 categorical columns → should infer Heatmap
        let block = ChartBlock {
            source: "departments.csv".to_string(),
            chart_type: None, // No explicit type
            x_col: None,
            y_col: None,
            color_col: None,
            title: None,
            filter: vec![],
        };

        let base_dir = std::path::Path::new("fixtures");
        let result = super::load_chart_data(&block, base_dir, &crate::theme::Theme::dark());
        assert!(result.is_ok(), "Should load: {:?}", result.err());
        assert!(
            matches!(result.unwrap(), crate::render::ChartData::Heatmap(_)),
            "Cat×Cat data should infer Heatmap"
        );
    }

    #[test]
    fn test_load_chart_data_infers_line_for_temporal() {
        // sales.csv has temporal x + quantitative y → should infer Line
        let block = ChartBlock {
            source: "sales.csv".to_string(),
            chart_type: None,
            x_col: None,
            y_col: None,
            color_col: None,
            title: None,
            filter: vec![],
        };

        let base_dir = std::path::Path::new("fixtures");
        let result = super::load_chart_data(&block, base_dir, &crate::theme::Theme::dark());
        assert!(result.is_ok(), "Should load: {:?}", result.err());
        assert!(
            matches!(result.unwrap(), crate::render::ChartData::Line(_)),
            "Temporal×Quant data should infer Line"
        );
    }

    #[test]
    fn test_jump_to_slide_with_digits() {
        let pres = Presentation {
            slides: (0..10)
                .map(|i| Slide {
                    title: Some(format!("Slide {}", i + 1)),
                    content: vec![],
                })
                .collect(),
        };
        let mut app = PresentApp::new(pres, std::path::PathBuf::new(), crate::theme::Theme::dark());
        assert_eq!(app.current_slide, 0);

        // Type "5" then Enter → jump to slide 5 (index 4)
        app.handle_key(KeyCode::Char('5'));
        app.handle_key(KeyCode::Enter);
        assert_eq!(
            app.current_slide, 4,
            "Should jump to slide 5 (0-indexed: 4)"
        );
    }

    #[test]
    fn test_jump_to_slide_clamped_to_max() {
        let pres = Presentation {
            slides: (0..3)
                .map(|i| Slide {
                    title: Some(format!("Slide {}", i + 1)),
                    content: vec![],
                })
                .collect(),
        };
        let mut app = PresentApp::new(pres, std::path::PathBuf::new(), crate::theme::Theme::dark());

        // Type "99" then Enter → clamped to last slide (index 2)
        app.handle_key(KeyCode::Char('9'));
        app.handle_key(KeyCode::Char('9'));
        app.handle_key(KeyCode::Enter);
        assert_eq!(app.current_slide, 2, "Should be clamped to last slide");
    }

    #[test]
    fn test_jump_to_slide_zero_goes_to_first() {
        let pres = Presentation {
            slides: (0..5)
                .map(|i| Slide {
                    title: Some(format!("Slide {}", i + 1)),
                    content: vec![],
                })
                .collect(),
        };
        let mut app = PresentApp::new(pres, std::path::PathBuf::new(), crate::theme::Theme::dark());
        app.current_slide = 3;

        // Type "0" then Enter → clamped to 0 (first slide)
        app.handle_key(KeyCode::Char('0'));
        app.handle_key(KeyCode::Enter);
        assert_eq!(app.current_slide, 0, "Slide 0 (or 1) should go to first");
    }

    #[test]
    fn test_jump_escape_clears_buffer() {
        let pres = Presentation {
            slides: (0..10)
                .map(|i| Slide {
                    title: Some(format!("Slide {}", i + 1)),
                    content: vec![],
                })
                .collect(),
        };
        let mut app = PresentApp::new(pres, std::path::PathBuf::new(), crate::theme::Theme::dark());
        app.current_slide = 2;

        // Type "7" then Escape → should NOT jump, stay at 2
        app.handle_key(KeyCode::Char('7'));
        // Now the buffer has "7", but 'q' should quit, not jump
        // Actually Esc should clear buffer without quitting (when buffer is non-empty)
        app.handle_key(KeyCode::Esc);
        assert_eq!(app.current_slide, 2, "Escape should cancel jump");
        assert!(
            !app.should_quit,
            "Escape with pending input should not quit"
        );
    }

    #[test]
    fn test_parse_inline_spans_plain_text() {
        let spans = parse_inline_spans("hello world");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "hello world");
    }

    #[test]
    fn test_parse_inline_spans_bold() {
        let spans = parse_inline_spans("before **bold** after");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].content, "before ");
        assert_eq!(spans[1].content, "bold");
        assert!(spans[1].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(spans[2].content, " after");
    }

    #[test]
    fn test_parse_inline_spans_italic() {
        let spans = parse_inline_spans("some *italic* text");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[1].content, "italic");
        assert!(spans[1].style.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn test_parse_inline_spans_code() {
        use ratatui::style::Color;
        let spans = parse_inline_spans("use `code` here");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[1].content, "code");
        assert_eq!(spans[1].style.fg, Some(Color::Yellow));
    }
}
