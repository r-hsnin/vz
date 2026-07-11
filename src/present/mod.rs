use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::path::{Path, PathBuf};

use crate::chart::data_builder;
use crate::chart::selector::ChartType;
use crate::cli::AggFunction;

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
}

impl PresentApp {
    pub fn new(presentation: Presentation, base_dir: std::path::PathBuf) -> Self {
        Self {
            presentation,
            current_slide: 0,
            should_quit: false,
            base_dir,
            input_buffer: String::new(),
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
pub fn run_present(path: &Path) -> Result<()> {
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
    let mut app = PresentApp::new(presentation, base_dir);

    loop {
        terminal.draw(|frame| draw_slide(frame, &app))?;

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

fn draw_slide(frame: &mut Frame, app: &PresentApp) {
    let chunks = Layout::vertical([
        Constraint::Min(3),    // content
        Constraint::Length(1), // footer
    ])
    .split(frame.area());

    if let Some(slide) = app.current_slide() {
        let content_area = chunks[0];

        // Split content area for title + body
        let inner_chunks = Layout::vertical([
            Constraint::Length(if slide.title.is_some() { 3 } else { 0 }),
            Constraint::Min(1),
        ])
        .split(content_area);

        // Title
        if let Some(ref title) = slide.title {
            let title_widget = Paragraph::new(Line::from(vec![Span::styled(
                title.clone(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]))
            .block(Block::default().borders(Borders::BOTTOM));
            frame.render_widget(title_widget, inner_chunks[0]);
        }

        // Body elements
        let body_area = if slide.title.is_some() {
            inner_chunks[1]
        } else {
            inner_chunks[0]
        };

        render_slide_body(frame, &slide.content, body_area, &app.base_dir);
    }

    // Footer with slide indicator
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" {} ", app.slide_indicator()),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw("  "),
        Span::styled("←/→", Style::default().fg(Color::Yellow)),
        Span::raw(" navigate  "),
        Span::styled("q", Style::default().fg(Color::Yellow)),
        Span::raw(" quit"),
    ]));
    frame.render_widget(footer, chunks[1]);
}

fn render_slide_body(
    frame: &mut Frame,
    elements: &[SlideElement],
    area: ratatui::layout::Rect,
    base_dir: &Path,
) {
    let constraints: Vec<Constraint> = elements.iter().map(element_constraint).collect();

    if constraints.is_empty() {
        return;
    }

    let chunks = Layout::vertical(constraints).spacing(1).split(area);

    for (i, element) in elements.iter().enumerate() {
        if i >= chunks.len() {
            break;
        }
        render_element(frame, element, chunks[i], base_dir);
    }
}

/// Compute the layout constraint for a single slide element.
fn element_constraint(el: &SlideElement) -> Constraint {
    match el {
        SlideElement::Chart(_) => Constraint::Min(10),
        SlideElement::Text(_) => Constraint::Length(2),
        SlideElement::Bullets(items) => Constraint::Length(items.len() as u16 + 1),
        SlideElement::Code { content, .. } => {
            Constraint::Length(content.lines().count() as u16 + 2)
        }
        SlideElement::Heading { .. } => Constraint::Length(2),
        SlideElement::OrderedList(items) => Constraint::Length(items.len() as u16 + 1),
    }
}

/// Render a single slide element into the given area.
fn render_element(
    frame: &mut Frame,
    element: &SlideElement,
    area: ratatui::layout::Rect,
    base_dir: &Path,
) {
    match element {
        SlideElement::Text(text) => {
            let spans = parse_inline_spans(text);
            let paragraph = Paragraph::new(Line::from(spans)).wrap(Wrap { trim: true });
            frame.render_widget(paragraph, area);
        }
        SlideElement::Bullets(items) => {
            let lines: Vec<Line> = items
                .iter()
                .map(|item| {
                    Line::from(vec![
                        Span::styled("  • ", Style::default().fg(Color::Yellow)),
                        Span::raw(item.clone()),
                    ])
                })
                .collect();
            frame.render_widget(Paragraph::new(lines), area);
        }
        SlideElement::Chart(chart_block) => {
            render_chart_placeholder(frame, chart_block, area, base_dir);
        }
        SlideElement::Code { language, content } => {
            render_code_block(frame, language.as_deref(), content, area);
        }
        SlideElement::Heading { level, text } => {
            let style = match level {
                2 => Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
                _ => Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            };
            let paragraph = Paragraph::new(Line::from(Span::styled(text.clone(), style)));
            frame.render_widget(paragraph, area);
        }
        SlideElement::OrderedList(items) => {
            let lines: Vec<Line> = items
                .iter()
                .enumerate()
                .map(|(idx, item)| {
                    Line::from(vec![
                        Span::styled(
                            format!("  {}. ", idx + 1),
                            Style::default().fg(Color::Yellow),
                        ),
                        Span::raw(item.clone()),
                    ])
                })
                .collect();
            frame.render_widget(Paragraph::new(lines), area);
        }
    }
}

/// Render a fenced code block with syntax-highlighted border.
fn render_code_block(
    frame: &mut Frame,
    language: Option<&str>,
    content: &str,
    area: ratatui::layout::Rect,
) {
    let title = language.map(|l| format!(" {} ", l)).unwrap_or_default();
    let code_lines: Vec<Line> = content
        .lines()
        .map(|l| {
            Line::from(Span::styled(
                l.to_string(),
                Style::default().fg(Color::Green),
            ))
        })
        .collect();
    let block_widget = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let paragraph = Paragraph::new(code_lines).block(block_widget);
    frame.render_widget(paragraph, area);
}

fn render_chart_placeholder(
    frame: &mut Frame,
    block: &ChartBlock,
    area: ratatui::layout::Rect,
    base_dir: &Path,
) {
    use crate::render::ChartWidget;

    if let Ok(chart_data) = load_chart_data(block, base_dir) {
        frame.render_widget(ChartWidget(&chart_data), area);
    } else {
        // Fallback: show chart block info
        let info = format!(
            "📊 Chart: source={}, type={:?}",
            block.source,
            block.chart_type.unwrap_or(ChartType::Line)
        );
        let placeholder =
            Paragraph::new(info).block(Block::default().title("Chart").borders(Borders::ALL));
        frame.render_widget(placeholder, area);
    }
}

/// Resolve the chart source file path relative to the markdown file's directory.
fn resolve_chart_source_path(source: &str, base_dir: &Path) -> PathBuf {
    let source_path = Path::new(source);
    if source_path.is_absolute() {
        return source_path.to_path_buf();
    }
    let relative_to_md = base_dir.join(source_path);
    if relative_to_md.exists() {
        relative_to_md
    } else {
        source_path.to_path_buf()
    }
}

fn load_chart_data(block: &ChartBlock, base_dir: &Path) -> Result<crate::render::ChartData> {
    let path = resolve_chart_source_path(&block.source, base_dir);

    let mut data = crate::loader::load_data(&path).with_context(|| {
        format!(
            "Chart source not found: {} (tried: {:?})",
            block.source, path
        )
    })?;

    // Apply filter if specified in chart block.
    if !block.filter.is_empty() {
        let predicates: Vec<crate::filter::Predicate> = block
            .filter
            .iter()
            .map(|expr| crate::filter::parse_predicate(expr))
            .collect::<Result<Vec<_>>>()?;
        data = crate::filter::filter_data(data, &predicates)?;
    }

    let headers = &data.headers;
    let rows = &data.rows;

    let chart_type = block
        .chart_type
        .unwrap_or_else(|| infer_chart_type_from_data(headers, rows, block));

    let axes = data_builder::ResolvedAxes::from_explicit(
        block.x_col.as_deref(),
        block.y_col.as_deref(),
        block.color_col.as_deref(),
        headers,
    );
    build_chart_data_for_type(chart_type, block, rows, &axes)
}

/// Infer chart type from data when not explicitly specified in chart block.
fn infer_chart_type_from_data(
    headers: &[String],
    rows: &[Vec<String>],
    block: &ChartBlock,
) -> ChartType {
    let h_refs: Vec<&str> = headers.iter().map(|s| s.as_str()).collect();
    let row_refs: Vec<Vec<&str>> = rows
        .iter()
        .map(|r| r.iter().map(|s| s.as_str()).collect())
        .collect();
    let schema = crate::infer::infer_schema(&h_refs, &row_refs);
    let x_hint = block.x_col.as_deref();
    let y_hint = block.y_col.as_deref();
    crate::chart::select_chart(&schema, x_hint, y_hint)
        .map(|rec| rec.chart_type)
        .unwrap_or(ChartType::Line)
}

/// Build the appropriate ChartData variant from resolved parameters.
fn build_chart_data_for_type(
    chart_type: ChartType,
    block: &ChartBlock,
    rows: &[Vec<String>],
    cols: &data_builder::ResolvedAxes,
) -> Result<crate::render::ChartData> {
    use crate::render::ChartData;

    match chart_type {
        ChartType::Heatmap => {
            let title = block
                .title
                .clone()
                .unwrap_or_else(|| format!("{} × {}", cols.x_label, cols.y_label));
            let data = data_builder::build_heatmap_data(rows, cols.x_idx, cols.y_idx, Some(title));
            Ok(ChartData::Heatmap(data))
        }
        ChartType::Bar => {
            let (data, _) = data_builder::aggregate_bar(
                rows,
                cols.x_idx,
                cols.y_idx,
                block.title.clone(),
                cols.y_label.clone(),
                AggFunction::Sum,
            );
            Ok(ChartData::Bar(data))
        }
        ChartType::Histogram => {
            let data = data_builder::build_histogram(
                rows,
                cols.x_idx,
                block.title.clone(),
                cols.x_label.clone(),
            );
            Ok(ChartData::Histogram(data))
        }
        ChartType::Line | ChartType::Scatter => {
            let config = data_builder::build_chart_config(
                rows,
                cols.x_idx,
                cols.y_idx,
                cols.color_idx,
                cols.x_label.clone(),
                cols.y_label.clone(),
                block.title.clone(),
            );
            if chart_type == ChartType::Scatter {
                Ok(ChartData::Scatter(config))
            } else {
                Ok(ChartData::Line(config))
            }
        }
    }
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
        let mut app = PresentApp::new(pres, std::path::PathBuf::new());
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
        let mut app = PresentApp::new(pres, std::path::PathBuf::new());
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
        let mut app = PresentApp::new(pres, std::path::PathBuf::new());
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
        let result = super::load_chart_data(&block, base_dir);
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
        let result = super::load_chart_data(&block, base_dir);
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
        let result = super::load_chart_data(&block, base_dir);
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
        let result = super::load_chart_data(&block, base_dir);
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
        let result = super::load_chart_data(&block, base_dir);
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
        let result = super::load_chart_data(&block, base_dir);
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
        let result = super::load_chart_data(&block, base_dir);
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
        let mut app = PresentApp::new(pres, std::path::PathBuf::new());
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
        let mut app = PresentApp::new(pres, std::path::PathBuf::new());

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
        let mut app = PresentApp::new(pres, std::path::PathBuf::new());
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
        let mut app = PresentApp::new(pres, std::path::PathBuf::new());
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
}
