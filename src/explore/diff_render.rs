//! Rendering functions for the Diff Explore mode TUI.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table},
};

use super::ViewMode;
use super::diff::{DiffData, DiffExploreApp};

pub fn draw_diff_ui(frame: &mut Frame, app: &DiffExploreApp) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // header
        Constraint::Min(10),   // chart or table
        Constraint::Length(3), // status bar
    ])
    .split(frame.area());

    let header = build_diff_header(app);
    frame.render_widget(header, chunks[0]);

    match app.view_mode {
        ViewMode::Chart => render_diff_chart(frame, app, chunks[1]),
        ViewMode::Table => render_diff_table(frame, app, chunks[1]),
    }

    let status = build_diff_status_bar(app);
    frame.render_widget(status, chunks[2]);

    if app.show_help {
        render_diff_help_overlay(frame);
    }
}

fn render_diff_chart(frame: &mut Frame, app: &DiffExploreApp, area: ratatui::layout::Rect) {
    use crate::render::{BarChartData, ChartData, ChartWidget};

    match &app.diff_data {
        DiffData::Categorical(result) => {
            let entries = app.sorted_entries();
            let labels: Vec<String> = entries
                .iter()
                .map(|e| {
                    let dir = if e.delta > 0.0 {
                        "▲"
                    } else if e.delta < 0.0 {
                        "▼"
                    } else {
                        "─"
                    };
                    let pct = e
                        .pct_change
                        .map(|p| format!("{:+.0}%", p))
                        .unwrap_or_else(|| "new".to_string());
                    format!("{} {} {}", e.label, dir, pct)
                })
                .collect();
            let values: Vec<f64> = entries.iter().map(|e| e.after).collect();

            let title = format!(
                "Diff: {} by {} ({} vs {})",
                result.y_column, result.x_column, app.before_name, app.after_name
            );

            let bar_data = BarChartData {
                labels,
                values,
                title: Some(title),
                y_label: result.y_column.clone(),
                show_labels: true,
                series_colors: vec![],
                axis_color: Some(app.theme.axis_color),
            };
            let chart_data = ChartData::Bar(bar_data);
            frame.render_widget(ChartWidget(&chart_data), area);
        }
        DiffData::Temporal(ts) => {
            use crate::render::ChartWidget;
            let title = format!(
                "Diff: {} over {} ({} vs {})",
                ts.y_column, ts.x_column, app.before_name, app.after_name
            );
            // Canonical temporal-diff config; interactive TUI keeps full
            // union labels (no terminal-width fitting here).
            let mut config = crate::chart::data_builder::build_diff_line_config(
                &ts.before,
                &ts.after,
                &ts.x_labels,
                &ts.x_column,
                &ts.y_column,
                Some(title),
            );
            config.series[0].name = app.before_name.clone();
            config.series[1].name = app.after_name.clone();
            config.axis_color = Some(app.theme.axis_color);
            config.label_color = Some(app.theme.label_color);
            let chart_data = ChartData::Line(config);
            frame.render_widget(ChartWidget(&chart_data), area);
        }
    }
}

fn render_diff_table(frame: &mut Frame, app: &DiffExploreApp, area: ratatui::layout::Rect) {
    let x_col_name = app.x_column().to_string();
    let headers = [&x_col_name[..], "Before", "After", "Δ", "%Change", "Dir"];

    let header_row = Row::new(
        headers
            .iter()
            .map(|h| ratatui::text::Text::styled(*h, Style::default().fg(Color::White))),
    )
    .style(Style::default().add_modifier(Modifier::BOLD))
    .height(1);

    let rows = app.table_rows();
    let total_rows = rows.len();
    let visible_height = area.height.saturating_sub(3) as usize;
    let end = (app.table_offset + visible_height).min(total_rows);
    let start = app.table_offset.min(total_rows);

    let visible_rows: Vec<Row> = rows[start..end]
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let cells: Vec<Cell> = row
                .iter()
                .enumerate()
                .map(|(ci, val)| {
                    if (1..=4).contains(&ci) {
                        // Numeric columns: right-align
                        Cell::new(
                            ratatui::text::Text::from(val.as_str()).alignment(Alignment::Right),
                        )
                    } else if ci == 5 {
                        // Direction column: color it
                        let color = match val.as_str() {
                            "▲" => Color::Green,
                            "▼" => Color::Red,
                            _ => Color::DarkGray,
                        };
                        Cell::new(ratatui::text::Text::from(val.as_str()))
                            .style(Style::default().fg(color))
                    } else {
                        Cell::new(val.as_str())
                    }
                })
                .collect();
            let r = Row::new(cells);
            if i == 0 {
                r.style(Style::default().fg(Color::Yellow))
            } else {
                r
            }
        })
        .collect();

    let widths = vec![
        Constraint::Length(16), // label
        Constraint::Length(10), // before
        Constraint::Length(10), // after
        Constraint::Length(10), // delta
        Constraint::Length(8),  // pct
        Constraint::Length(4),  // dir
    ];

    let table = Table::new(visible_rows, widths).header(header_row).block(
        Block::default()
            .title(format!(
                " Diff Table ({}-{} of {}) ",
                start + 1,
                end,
                total_rows
            ))
            .borders(Borders::ALL),
    );

    frame.render_widget(table, area);
}

fn build_diff_header(app: &DiffExploreApp) -> Paragraph<'static> {
    let chart_type = if app.is_temporal() { "Line" } else { "Bar" };
    let overall = app
        .overall_pct()
        .map(|p| {
            let dir = if p > 0.0 {
                "▲"
            } else if p < 0.0 {
                "▼"
            } else {
                "─"
            };
            format!(" │ {} {:+.1}%", dir, p)
        })
        .unwrap_or_default();

    let text = Line::from(vec![
        Span::styled(
            " vz ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "DIFF ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            "│ {} │ X: {} │ Y: {} │ {} vs {} │ {} entries{}",
            chart_type,
            app.x_column(),
            app.y_column(),
            app.before_name,
            app.after_name,
            app.entry_count(),
            overall,
        )),
    ]);

    Paragraph::new(text).block(Block::default().borders(Borders::BOTTOM))
}

fn build_diff_status_bar(app: &DiffExploreApp) -> Paragraph<'static> {
    let sort_label = match app.sort_order {
        None | Some(crate::chart::selector::SortOrder::None) => "off",
        Some(crate::chart::selector::SortOrder::Desc) => "desc",
        Some(crate::chart::selector::SortOrder::Asc) => "asc",
    };

    let bindings: Vec<(&str, &str)> = vec![
        ("s", sort_label),
        ("d", "data"),
        ("y", "yank"),
        ("?", "help"),
        ("q", "quit"),
    ];

    let mut spans: Vec<Span<'static>> = vec![Span::raw(" ".to_string())];
    for (key, desc) in bindings {
        spans.push(Span::styled(
            key.to_string(),
            Style::default().fg(Color::Yellow),
        ));
        spans.push(Span::raw(format!("={} ", desc)));
    }

    let text = Line::from(spans);
    let mut lines = vec![text];
    if let Some(ref msg) = app.status_message {
        lines.push(Line::from(Span::styled(
            format!(" ⚠ {}", msg),
            Style::default().fg(Color::Yellow),
        )));
    }

    Paragraph::new(lines).block(Block::default().borders(Borders::TOP))
}

fn render_diff_help_overlay(frame: &mut Frame) {
    let area = frame.area();
    let help_width = 44.min(area.width.saturating_sub(4));
    let help_height = 14.min(area.height.saturating_sub(2));
    let x = (area.width.saturating_sub(help_width)) / 2;
    let y = (area.height.saturating_sub(help_height)) / 2;
    let popup = ratatui::layout::Rect::new(x, y, help_width, help_height);

    frame.render_widget(Clear, popup);

    let help_text = vec![
        Line::from(Span::styled(
            " Diff Explore ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        Line::raw(" j / k (↑/↓)   Scroll table"),
        Line::raw(" G / End        Jump to last row"),
        Line::raw(" g / Home       Jump to first row"),
        Line::raw(" PgDn / PgUp    Page scroll"),
        Line::raw(" s              Cycle sort (desc/asc/off)"),
        Line::raw(" y              Yank equivalent command"),
        Line::raw(" d / Tab        Toggle chart ↔ table"),
        Line::raw(" ?              Show/hide this help"),
        Line::raw(" q / Esc        Quit"),
        Line::raw(""),
        Line::from(Span::styled(
            " Press any key to close ",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(help_text).alignment(Alignment::Left).block(
        Block::default()
            .title(" Help ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(paragraph, popup);
}
