//! Rendering functions for the Explore mode TUI.

use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table},
};

use crate::chart::selector::ChartType;
use crate::cli::{AggFunction, SortOrder};

use super::{ExploreApp, ViewMode};

pub fn draw_ui(frame: &mut Frame, app: &ExploreApp) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // header
        Constraint::Min(10),   // chart or table
        Constraint::Length(3), // status bar
    ])
    .split(frame.area());

    // Header
    let header = build_header(app);
    frame.render_widget(header, chunks[0]);

    match app.view_mode {
        ViewMode::Chart => render_chart(frame, app, chunks[1]),
        ViewMode::Table => render_table(frame, app, chunks[1]),
    }

    // Status bar
    let status = build_status_bar(app);
    frame.render_widget(status, chunks[2]);

    // Help overlay (rendered last so it appears on top)
    if app.show_help {
        render_help_overlay(frame);
    }
}

fn render_chart(frame: &mut Frame, app: &ExploreApp, area: ratatui::layout::Rect) {
    use crate::render::{ChartData, ChartWidget};

    let chart_type = app.effective_chart_type();
    let chart_data = match chart_type {
        ChartType::Line => ChartData::Line(app.build_chart_config()),
        ChartType::Scatter => ChartData::Scatter(app.build_chart_config()),
        ChartType::Bar => ChartData::Bar(app.build_bar_data()),
        ChartType::Heatmap => ChartData::Heatmap(app.build_heatmap_data()),
        ChartType::Histogram => ChartData::Histogram(app.build_histogram_data()),
    };
    frame.render_widget(ChartWidget(&chart_data), area);
}

fn render_table(frame: &mut Frame, app: &ExploreApp, area: ratatui::layout::Rect) {
    let col_count = app.schema.columns.len();
    let header_cells: Vec<&str> = app.schema.columns.iter().map(|c| c.name.as_str()).collect();

    let header_row = Row::new(header_cells.iter().map(|h| {
        let idx = app.schema.columns.iter().position(|c| c.name == *h);
        let style = if idx == Some(app.selected_x) || idx == Some(app.selected_y) {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        ratatui::text::Text::styled(*h, style)
    }))
    .style(Style::default().add_modifier(Modifier::BOLD))
    .height(1);

    let visible_height = area.height.saturating_sub(3) as usize;
    let end = (app.table_offset + visible_height).min(app.data.len());
    let visible_rows: Vec<Row> = app.data[app.table_offset..end]
        .iter()
        .map(|row| {
            let cells: Vec<String> = (0..col_count)
                .map(|i| row.get(i).cloned().unwrap_or_default())
                .collect();
            Row::new(cells)
        })
        .collect();

    let widths: Vec<Constraint> = (0..col_count)
        .map(|_| Constraint::Percentage((100 / col_count.max(1)) as u16))
        .collect();

    let table = Table::new(visible_rows, widths)
        .header(header_row)
        .block(
            Block::default()
                .title(format!(
                    " Data ({}-{} of {}) ",
                    app.table_offset + 1,
                    end,
                    app.data.len()
                ))
                .borders(Borders::ALL),
        )
        .row_highlight_style(Style::default().fg(Color::Yellow));

    frame.render_widget(table, area);
}

fn build_header(app: &ExploreApp) -> Paragraph<'static> {
    let x_name = app
        .schema
        .columns
        .get(app.selected_x)
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "?".to_string());
    let y_name = app
        .schema
        .columns
        .get(app.selected_y)
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "?".to_string());
    let x_type = app
        .schema
        .columns
        .get(app.selected_x)
        .map(|c| c.data_type.to_string())
        .unwrap_or_else(|| "?".to_string());
    let y_type = app
        .schema
        .columns
        .get(app.selected_y)
        .map(|c| c.data_type.to_string())
        .unwrap_or_else(|| "?".to_string());
    let chart_type = app.effective_chart_type();
    let row_count = app.data.len();

    let color_info = app
        .selected_color
        .and_then(|i| app.schema.columns.get(i))
        .map(|c| format!(" │ C: {}", c.name))
        .unwrap_or_default();

    let text = Line::from(vec![
        Span::styled(
            " vz ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            "│ X: {} ({}) │ Y: {} ({}) │ {} │ {} rows{}",
            x_name, x_type, y_name, y_type, chart_type, row_count, color_info
        )),
    ]);

    Paragraph::new(text).block(Block::default().borders(Borders::BOTTOM))
}

fn render_help_overlay(frame: &mut Frame) {
    use ratatui::layout::Alignment;
    use ratatui::widgets::Clear;

    let area = frame.area();
    let help_width = 44.min(area.width.saturating_sub(4));
    let help_height = 18.min(area.height.saturating_sub(2));
    let x = (area.width.saturating_sub(help_width)) / 2;
    let y = (area.height.saturating_sub(help_height)) / 2;
    let popup = ratatui::layout::Rect::new(x, y, help_width, help_height);

    frame.render_widget(Clear, popup);

    let help_text = vec![
        Line::from(Span::styled(
            " Keybindings ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        Line::raw(" h / l (←/→)   Change X axis column"),
        Line::raw(" j / k (↑/↓)   Change Y / scroll table"),
        Line::raw(" G / End        Jump to last row (table)"),
        Line::raw(" g / Home       Jump to first row (table)"),
        Line::raw(" PgDn / PgUp    Page scroll (table)"),
        Line::raw(" c              Cycle color/group column"),
        Line::raw(" s              Cycle sort (desc/asc/off)"),
        Line::raw(" a              Cycle aggregation function"),
        Line::raw(" y              Yank equivalent command"),
        Line::raw(" 1-4            Force chart type"),
        Line::raw("                (Line/Bar/Scatter/Hist)"),
        Line::raw(" 0              Auto chart type"),
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

fn build_column_display(app: &ExploreApp) -> String {
    app.schema
        .columns
        .iter()
        .enumerate()
        .map(|(i, col)| {
            let marker = if i == app.selected_x && i == app.selected_y {
                "xy"
            } else if i == app.selected_x {
                "x"
            } else if i == app.selected_y {
                "y"
            } else {
                " "
            };
            format!("[{}]{}", marker, col.name)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn build_binding_spans(app: &ExploreApp) -> Vec<Span<'static>> {
    let color_label = match app.selected_color {
        Some(idx) => app
            .schema
            .columns
            .get(idx)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "?".to_string()),
        None => "off".to_string(),
    };

    let sort_label = match app.sort_order {
        None | Some(SortOrder::None) => "off".to_string(),
        Some(SortOrder::Desc) => "desc".to_string(),
        Some(SortOrder::Asc) => "asc".to_string(),
    };

    let agg_label = match app.agg_function {
        AggFunction::Sum => "sum",
        AggFunction::Mean => "mean",
        AggFunction::Count => "count",
        AggFunction::Max => "max",
        AggFunction::Min => "min",
    }
    .to_string();

    let bindings: &[(&str, String)] = &[
        ("h/l", "X".to_string()),
        ("j/k", "Y".to_string()),
        ("c", color_label),
        ("s", sort_label),
        ("a", agg_label),
        ("1-4", "type".to_string()),
        ("0", "auto".to_string()),
        ("d", "data".to_string()),
        ("?", "help".to_string()),
        ("q", "quit".to_string()),
    ];

    let mut spans: Vec<Span<'static>> = vec![Span::raw(" ".to_string())];
    for (key, desc) in bindings {
        spans.push(Span::styled(
            key.to_string(),
            Style::default().fg(Color::Yellow),
        ));
        spans.push(Span::raw(format!("={} ", desc)));
    }
    spans
}

fn build_status_bar(app: &ExploreApp) -> Paragraph<'static> {
    let col_display = build_column_display(app);
    let mut spans = build_binding_spans(app);
    spans.push(Span::raw("│ ".to_string()));
    spans.push(Span::styled(
        col_display,
        Style::default().fg(Color::DarkGray),
    ));

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infer::types::{ColumnMeta, DataType, Schema};

    fn make_test_app() -> ExploreApp {
        let schema = Schema {
            columns: vec![
                ColumnMeta {
                    name: "date".to_string(),
                    data_type: DataType::Temporal,
                    null_count: 0,
                    sample_size: 2,
                },
                ColumnMeta {
                    name: "city".to_string(),
                    data_type: DataType::Categorical,
                    null_count: 0,
                    sample_size: 2,
                },
                ColumnMeta {
                    name: "revenue".to_string(),
                    data_type: DataType::Quantitative,
                    null_count: 0,
                    sample_size: 2,
                },
            ],
        };
        let data = vec![
            vec!["2024-01".into(), "Tokyo".into(), "1000".into()],
            vec!["2024-02".into(), "Osaka".into(), "1500".into()],
        ];
        ExploreApp::new(schema, data, crate::theme::Theme::dark())
    }

    #[test]
    fn test_build_column_display_shows_markers() {
        let app = make_test_app();
        let display = build_column_display(&app);
        // selected_x = 0 (date), selected_y = 2 (revenue)
        assert!(
            display.contains("[x]date"),
            "Should show x marker: {display}"
        );
        assert!(
            display.contains("[y]revenue"),
            "Should show y marker: {display}"
        );
        assert!(
            display.contains("[ ]city"),
            "Unselected should have space: {display}"
        );
    }

    #[test]
    fn test_build_binding_spans_contains_keys() {
        let app = make_test_app();
        let spans = build_binding_spans(&app);
        let text: String = spans.iter().map(|s| s.content.to_string()).collect();
        assert!(text.contains("h/l"), "Should contain h/l key: {text}");
        assert!(text.contains("j/k"), "Should contain j/k key: {text}");
        assert!(text.contains("q"), "Should contain quit key: {text}");
        assert!(text.contains("?"), "Should contain help key: {text}");
    }

    #[test]
    fn test_build_binding_spans_shows_color_off() {
        let app = make_test_app();
        let spans = build_binding_spans(&app);
        let text: String = spans.iter().map(|s| s.content.to_string()).collect();
        assert!(
            text.contains("off"),
            "No color selected should show 'off': {text}"
        );
    }

    #[test]
    fn test_build_binding_spans_shows_color_name() {
        let mut app = make_test_app();
        app.selected_color = Some(1); // city
        let spans = build_binding_spans(&app);
        let text: String = spans.iter().map(|s| s.content.to_string()).collect();
        assert!(
            text.contains("city"),
            "Color column name should appear: {text}"
        );
    }
}
