use anyhow::Result;
use crossterm::event::{self, Event, KeyEventKind};

// Re-export for tests.rs (`use super::*`)
#[cfg(test)]
use crossterm::event::KeyCode;
use std::io::IsTerminal;

use crate::chart::selector::ChartType;
use crate::cli::{AggFunction, SortOrder};
use crate::infer::types::Schema;

/// View mode for the Explore TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Chart,
    Table,
}

/// Application state for Explore mode.
pub struct ExploreApp {
    pub schema: Schema,
    pub data: Vec<Vec<String>>,
    pub selected_x: usize,
    pub selected_y: usize,
    pub selected_color: Option<usize>,
    pub chart_type_override: Option<ChartType>,
    pub should_quit: bool,
    pub view_mode: ViewMode,
    pub table_offset: usize,
    /// Transient status message shown for one render cycle.
    pub status_message: Option<String>,
    /// Whether to show the help overlay.
    pub show_help: bool,
    /// Color theme for chart rendering.
    pub theme: crate::theme::Theme,
    /// Sort order for bar charts.
    pub sort_order: Option<SortOrder>,
    /// Aggregation function for bar charts.
    pub agg_function: AggFunction,
}

/// Run the Explore TUI app.
pub fn run_explore(
    schema: Schema,
    data: Vec<Vec<String>>,
    theme: crate::theme::Theme,
) -> Result<()> {
    if data.is_empty() {
        anyhow::bail!("No data rows to explore");
    }

    // In headless/CI environments, skip the TUI event loop to prevent hangs.
    // Tests use this to verify CLI flag parsing without entering the interactive loop.
    if std::env::var("VZ_TEST_HEADLESS").is_ok() {
        return Ok(());
    }

    if !std::io::stdout().is_terminal() {
        anyhow::bail!(
            "Explore mode requires an interactive terminal. \
             Cannot run in a pipe or non-TTY environment."
        );
    }

    let mut terminal = ratatui::init();
    let mut app = ExploreApp::new(schema, data, theme);

    loop {
        terminal.draw(|frame| render::draw_ui(frame, &app))?;
        app.status_message = None; // clear transient message after render

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

/// Run the Diff Explore TUI app.
pub fn run_explore_diff(
    diff_data: DiffData,
    before_name: String,
    after_name: String,
    theme: crate::theme::Theme,
) -> Result<()> {
    // In headless/CI environments, skip the TUI event loop.
    if std::env::var("VZ_TEST_HEADLESS").is_ok() {
        return Ok(());
    }

    if !std::io::stdout().is_terminal() {
        anyhow::bail!(
            "Explore mode requires an interactive terminal. \
             Cannot run in a pipe or non-TTY environment."
        );
    }

    let mut terminal = ratatui::init();
    let mut app = diff::DiffExploreApp::new(diff_data, before_name, after_name, theme);

    loop {
        terminal.draw(|frame| diff_render::draw_diff_ui(frame, &app))?;
        app.status_message = None;

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

mod app;
mod render;
mod state;

pub mod diff;
mod diff_render;

pub use diff::DiffData;

#[cfg(test)]
mod tests;
