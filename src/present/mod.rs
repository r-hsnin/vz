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
    /// Sort order for bar charts.
    pub sort: Option<crate::cli::SortOrder>,
    /// Aggregation function for bar charts.
    pub agg: Option<crate::cli::AggFunction>,
    /// Limit bar chart to top N categories.
    pub top: Option<usize>,
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
        .with_context(|| format!("Failed to read presentation file: {}", path.display()))?;
    let presentation = parse_presentation(&content);

    if presentation.slides.is_empty() {
        anyhow::bail!("No slides found in {}", path.display());
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
mod tests;
