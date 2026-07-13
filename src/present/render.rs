//! Slide rendering: draw_slide, render_slide_body, render_element, etc.

use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::path::Path;

use super::{ChartBlock, PresentApp, Slide, SlideElement, parse_inline_spans};
use crate::chart::selector::ChartType;

/// Draw the current slide to the terminal frame.
pub fn draw_slide(frame: &mut Frame, app: &PresentApp) {
    let chunks = Layout::vertical([
        Constraint::Min(3),    // content
        Constraint::Length(1), // footer
    ])
    .split(frame.area());

    if let Some(slide) = app.current_slide() {
        render_slide_content(frame, slide, chunks[0], &app.base_dir, &app.theme);
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

/// Render slide title + body content.
fn render_slide_content(
    frame: &mut Frame,
    slide: &Slide,
    content_area: ratatui::layout::Rect,
    base_dir: &Path,
    theme: &crate::theme::Theme,
) {
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

    render_slide_body(frame, &slide.content, body_area, base_dir, theme);
}

/// Render slide body elements with spacing.
fn render_slide_body(
    frame: &mut Frame,
    elements: &[SlideElement],
    area: ratatui::layout::Rect,
    base_dir: &Path,
    theme: &crate::theme::Theme,
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
        render_element(frame, element, chunks[i], base_dir, theme);
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
    theme: &crate::theme::Theme,
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
            render_chart_placeholder(frame, chart_block, area, base_dir, theme);
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

/// Render a chart from data or show a fallback placeholder.
fn render_chart_placeholder(
    frame: &mut Frame,
    block: &ChartBlock,
    area: ratatui::layout::Rect,
    base_dir: &Path,
    theme: &crate::theme::Theme,
) {
    use crate::render::ChartWidget;

    if let Ok(chart_data) = super::load_chart_data(block, base_dir, theme) {
        frame.render_widget(ChartWidget(&chart_data), area);
    } else {
        // Fallback: show chart block info
        let info = format!(
            "📊 Chart: source={}, type={}",
            block.source,
            block.chart_type.unwrap_or(ChartType::Line)
        );
        let placeholder =
            Paragraph::new(info).block(Block::default().title("Chart").borders(Borders::ALL));
        frame.render_widget(placeholder, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend, layout::Rect};

    fn make_app(slides: Vec<Slide>) -> PresentApp {
        use super::super::Presentation;
        PresentApp {
            presentation: Presentation { slides },
            current_slide: 0,
            should_quit: false,
            base_dir: std::path::PathBuf::from("."),
            input_buffer: String::new(),
            theme: crate::theme::Theme::default(),
        }
    }

    #[test]
    fn test_element_constraint_chart() {
        let el = SlideElement::Chart(ChartBlock {
            source: "data.csv".into(),
            chart_type: None,
            x_col: None,
            y_col: None,
            color_col: None,
            title: None,
            filter: vec![],
        });
        assert_eq!(element_constraint(&el), Constraint::Min(10));
    }

    #[test]
    fn test_element_constraint_text() {
        let el = SlideElement::Text("Hello world".into());
        assert_eq!(element_constraint(&el), Constraint::Length(2));
    }

    #[test]
    fn test_element_constraint_bullets() {
        let el = SlideElement::Bullets(vec!["a".into(), "b".into(), "c".into()]);
        assert_eq!(element_constraint(&el), Constraint::Length(4)); // 3 items + 1
    }

    #[test]
    fn test_element_constraint_code() {
        let el = SlideElement::Code {
            language: Some("rust".into()),
            content: "fn main() {\n    println!(\"hi\");\n}".into(),
        };
        // 3 lines + 2 (border)
        assert_eq!(element_constraint(&el), Constraint::Length(5));
    }

    #[test]
    fn test_element_constraint_heading() {
        let el = SlideElement::Heading {
            level: 2,
            text: "Title".into(),
        };
        assert_eq!(element_constraint(&el), Constraint::Length(2));
    }

    #[test]
    fn test_element_constraint_ordered_list() {
        let el = SlideElement::OrderedList(vec!["one".into(), "two".into()]);
        assert_eq!(element_constraint(&el), Constraint::Length(3)); // 2 items + 1
    }

    #[test]
    fn test_draw_slide_renders_footer() {
        let slide = Slide {
            title: Some("Test Slide".into()),
            content: vec![SlideElement::Text("Body text".into())],
        };
        let app = make_app(vec![slide]);
        let backend = TestBackend::new(60, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| draw_slide(frame, &app)).unwrap();
        let buffer = terminal.backend().buffer().clone();
        // Footer should contain slide indicator "1/1"
        let content = buffer_to_string(&buffer);
        assert!(
            content.contains("1/1"),
            "Footer should show slide indicator"
        );
        assert!(
            content.contains("navigate"),
            "Footer should show navigation hint"
        );
    }

    #[test]
    fn test_draw_slide_renders_title() {
        let slide = Slide {
            title: Some("My Title".into()),
            content: vec![],
        };
        let app = make_app(vec![slide]);
        let backend = TestBackend::new(60, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| draw_slide(frame, &app)).unwrap();
        let buffer = terminal.backend().buffer().clone();
        let content = buffer_to_string(&buffer);
        assert!(
            content.contains("My Title"),
            "Should render slide title, got: {}",
            content
        );
    }

    #[test]
    fn test_render_code_block_shows_language() {
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render_code_block(frame, Some("python"), "print('hi')", Rect::new(0, 0, 40, 5));
            })
            .unwrap();
        let buffer = terminal.backend().buffer().clone();
        let content = buffer_to_string(&buffer);
        assert!(
            content.contains("python"),
            "Should show language label, got: {}",
            content
        );
    }

    /// Helper: convert buffer to a single string for searching.
    fn buffer_to_string(buf: &ratatui::buffer::Buffer) -> String {
        let area = buf.area();
        let mut s = String::new();
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                s.push_str(buf.cell((x, y)).map_or(" ", |c| c.symbol()));
            }
            s.push('\n');
        }
        s
    }
}
