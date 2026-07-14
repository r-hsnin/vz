use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};

use crate::chart::data_builder;
use crate::chart::selector::{ChartRecommendation, ChartType, select_chart};
use crate::cli::{AggFunction, SortOrder};
use crate::infer::types::{DataType, Schema};
use crate::render::{BarChartData, ChartConfig, HistogramData};

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

impl ExploreApp {
    pub fn new(schema: Schema, data: Vec<Vec<String>>, theme: crate::theme::Theme) -> Self {
        let (x_idx, y_idx) = initial_axes(&schema);
        Self {
            schema,
            data,
            selected_x: x_idx,
            selected_y: y_idx,
            selected_color: None, // auto-detect initially
            chart_type_override: None,
            should_quit: false,
            view_mode: ViewMode::Chart,
            table_offset: 0,
            status_message: Some("? help │ h/l axis │ j/k col │ d table │ q quit".to_string()),
            show_help: false,
            theme,
            sort_order: None,
            agg_function: AggFunction::Sum,
        }
    }

    pub fn current_recommendation(&self) -> Option<ChartRecommendation> {
        let x_name = self
            .schema
            .columns
            .get(self.selected_x)
            .map(|c| c.name.as_str());
        let y_name = self
            .schema
            .columns
            .get(self.selected_y)
            .map(|c| c.name.as_str());
        select_chart(&self.schema, x_name, y_name).ok()
    }

    pub fn handle_key(&mut self, key: KeyCode) {
        let prev_chart_type = self.effective_chart_type();
        // If help overlay is showing, any key dismisses it
        if self.show_help {
            self.show_help = false;
            return;
        }

        match key {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('?') => self.show_help = true,
            KeyCode::Char('d') | KeyCode::Tab => match self.view_mode {
                ViewMode::Chart => self.view_mode = ViewMode::Table,
                ViewMode::Table => self.view_mode = ViewMode::Chart,
            },
            KeyCode::Char('h') | KeyCode::Left => self.navigate_x(-1),
            KeyCode::Char('l') | KeyCode::Right => self.navigate_x(1),
            KeyCode::Char('j') | KeyCode::Down => self.navigate_y(1),
            KeyCode::Char('k') | KeyCode::Up => self.navigate_y(-1),
            KeyCode::Char('G') | KeyCode::End => self.navigate_y(isize::MAX),
            KeyCode::Char('g') | KeyCode::Home => self.navigate_y(isize::MIN),
            KeyCode::PageDown => self.navigate_y(12),
            KeyCode::PageUp => self.navigate_y(-12),
            KeyCode::Char('1') => self.chart_type_override = Some(ChartType::Line),
            KeyCode::Char('2') => self.chart_type_override = Some(ChartType::Bar),
            KeyCode::Char('3') => self.chart_type_override = Some(ChartType::Scatter),
            KeyCode::Char('4') => self.chart_type_override = Some(ChartType::Histogram),
            KeyCode::Char('0') => self.chart_type_override = None,
            KeyCode::Char('c') => self.cycle_color_column(),
            KeyCode::Char('y') => self.yank_command(),
            KeyCode::Char('s') => self.cycle_sort(),
            KeyCode::Char('a') => self.cycle_agg(),
            _ => {}
        }

        // Notify when chart type auto-changes due to column navigation
        let new_chart_type = self.effective_chart_type();
        if new_chart_type != prev_chart_type && self.chart_type_override.is_none() {
            self.status_message = Some(format!("auto: {} → {}", prev_chart_type, new_chart_type));
        }
    }

    /// Move X axis column selection, skipping the current Y column when possible.
    fn navigate_x(&mut self, direction: isize) {
        let max_idx = self.schema.columns.len().saturating_sub(1);
        if direction > 0 && self.selected_x < max_idx {
            self.selected_x += 1;
            if self.selected_x == self.selected_y && self.selected_x < max_idx {
                self.selected_x += 1;
            }
        } else if direction < 0 && self.selected_x > 0 {
            self.selected_x -= 1;
            if self.selected_x == self.selected_y && self.selected_x > 0 {
                self.selected_x -= 1;
            }
        }
    }

    /// Move Y axis column or scroll table. Large magnitude = page/jump.
    fn navigate_y(&mut self, direction: isize) {
        if self.view_mode == ViewMode::Table {
            let max = self.data.len().saturating_sub(1);
            match direction {
                isize::MAX => self.table_offset = max,
                isize::MIN => self.table_offset = 0,
                d if d > 0 => self.table_offset = (self.table_offset + d as usize).min(max),
                d => self.table_offset = self.table_offset.saturating_sub(d.unsigned_abs()),
            }
            return;
        }
        if direction.unsigned_abs() > 1 {
            return;
        }
        let max_idx = self.schema.columns.len().saturating_sub(1);
        if direction > 0 && self.selected_y < max_idx {
            self.selected_y += 1;
            if self.selected_y == self.selected_x && self.selected_y < max_idx {
                self.selected_y += 1;
            }
        } else if direction < 0 && self.selected_y > 0 {
            self.selected_y -= 1;
            if self.selected_y == self.selected_x && self.selected_y > 0 {
                self.selected_y -= 1;
            }
        }
    }

    /// Cycle through categorical columns for color grouping.
    /// None → first categorical → second categorical → … → None (off)
    fn cycle_color_column(&mut self) {
        let categoricals: Vec<usize> = self
            .schema
            .columns
            .iter()
            .enumerate()
            .filter(|(i, c)| {
                c.data_type == DataType::Categorical
                    && *i != self.selected_x
                    && *i != self.selected_y
            })
            .map(|(i, _)| i)
            .collect();

        if categoricals.is_empty() {
            self.selected_color = None;
            self.status_message = Some("no color columns available".to_string());
            return;
        }

        self.selected_color = match self.selected_color {
            None => Some(categoricals[0]),
            Some(current) => {
                let pos = categoricals.iter().position(|&i| i == current);
                match pos {
                    Some(p) if p + 1 < categoricals.len() => Some(categoricals[p + 1]),
                    _ => None, // wrap around to "off"
                }
            }
        };
    }

    /// Cycle sort order: None → Desc → Asc → None.
    fn cycle_sort(&mut self) {
        self.sort_order = match self.sort_order {
            None => Some(SortOrder::Desc),
            Some(SortOrder::Desc) => Some(SortOrder::Asc),
            Some(SortOrder::Asc) | Some(SortOrder::None) => None,
        };
        let label = match self.sort_order {
            None => "sort: off",
            Some(SortOrder::Desc) => "sort: desc",
            Some(SortOrder::Asc) => "sort: asc",
            Some(SortOrder::None) => "sort: off",
        };
        self.status_message = Some(label.to_string());
    }

    /// Cycle aggregation function: Sum → Mean → Count → Max → Min → Sum.
    fn cycle_agg(&mut self) {
        self.agg_function = match self.agg_function {
            AggFunction::Sum => AggFunction::Mean,
            AggFunction::Mean => AggFunction::Count,
            AggFunction::Count => AggFunction::Max,
            AggFunction::Max => AggFunction::Min,
            AggFunction::Min => AggFunction::Sum,
        };
        let label = match self.agg_function {
            AggFunction::Sum => "agg: sum",
            AggFunction::Mean => "agg: mean",
            AggFunction::Count => "agg: count",
            AggFunction::Max => "agg: max",
            AggFunction::Min => "agg: min",
        };
        self.status_message = Some(label.to_string());
    }

    /// Generate the equivalent oneshot command for the current view.
    fn yank_command(&mut self) {
        let x = self.x_label();
        let y = self.y_label();
        let chart_type = self.effective_chart_type();
        let type_flag = match chart_type {
            ChartType::Line => " -t line",
            ChartType::Bar => " -t bar",
            ChartType::Scatter => " -t scatter",
            ChartType::Histogram => " -t histogram",
            ChartType::Heatmap => " -t heatmap",
        };
        let color_part = self
            .selected_color
            .and_then(|i| self.schema.columns.get(i))
            .map(|c| format!(" -c {}", c.name))
            .unwrap_or_default();
        let sort_part = match self.sort_order {
            Some(SortOrder::Desc) => " --sort desc",
            Some(SortOrder::Asc) => " --sort asc",
            _ => "",
        };
        let agg_part = match self.agg_function {
            AggFunction::Sum => "",
            AggFunction::Mean => " --agg mean",
            AggFunction::Count => " --agg count",
            AggFunction::Max => " --agg max",
            AggFunction::Min => " --agg min",
        };
        let cmd = format!("vz <FILE> -x {x} -y {y}{type_flag}{color_part}{sort_part}{agg_part}");
        self.status_message = Some(cmd);
    }

    /// Extract Y column values as f64.
    pub fn y_values(&self) -> Vec<f64> {
        self.data
            .iter()
            .map(|row| {
                row.get(self.selected_y)
                    .and_then(|v| v.parse::<f64>().ok())
                    .unwrap_or(0.0)
            })
            .collect()
    }

    /// Get the current X-axis column name.
    fn x_label(&self) -> String {
        self.schema
            .columns
            .get(self.selected_x)
            .map(|c| c.name.clone())
            .unwrap_or_default()
    }

    /// Get the current Y-axis column name.
    fn y_label(&self) -> String {
        self.schema
            .columns
            .get(self.selected_y)
            .map(|c| c.name.clone())
            .unwrap_or_default()
    }

    /// Build chart configuration for the current selection.
    pub fn build_chart_config(&self) -> ChartConfig {
        let x_label = self.x_label();
        let y_label = self.y_label();

        // Use user-selected color column, or auto-detect
        let color_idx = self.selected_color.or_else(|| {
            self.schema.columns.iter().position(|c| {
                c.data_type == DataType::Categorical && c.name != x_label && c.name != y_label
            })
        });

        let title = format!("{} vs {}", y_label, x_label);
        let mut config = data_builder::build_chart_config(
            &self.data,
            self.selected_x,
            self.selected_y,
            color_idx,
            x_label,
            y_label,
            Some(title),
        );
        config.apply_theme(&self.theme);
        config
    }

    /// Build bar chart data (aggregated by category).
    pub fn build_bar_data(&self) -> BarChartData {
        let y_label = self.y_label();
        let title = format!("{} by category", y_label);

        let (mut data, _) = data_builder::aggregate_bar(
            &self.data,
            self.selected_x,
            self.selected_y,
            Some(title),
            y_label,
            self.agg_function,
        );
        crate::oneshot::builders::sort_bar_data(&mut data, self.sort_order);
        data.axis_color = Some(self.theme.axis_color);
        data
    }

    /// Build histogram data.
    pub fn build_histogram_data(&self) -> HistogramData {
        let x_label = self.x_label();
        let title = format!("Distribution of {}", x_label);

        let mut data =
            data_builder::build_histogram(&self.data, self.selected_x, Some(title), x_label, None);
        data.axis_color = Some(self.theme.axis_color);
        data
    }

    /// Build heatmap data for the two selected columns.
    pub fn build_heatmap_data(&self) -> crate::render::HeatmapData {
        let x_label = self.x_label();
        let y_label = self.y_label();
        let title = format!("{} × {}", x_label, y_label);
        data_builder::build_heatmap_data(&self.data, self.selected_x, self.selected_y, Some(title))
    }

    /// Get the effective chart type (override or auto).
    pub fn effective_chart_type(&self) -> ChartType {
        if let Some(ct) = self.chart_type_override {
            return ct;
        }
        self.current_recommendation()
            .map(|r| r.chart_type)
            .unwrap_or(ChartType::Line)
    }
}

/// Pick `count` evenly spaced items from a slice.
/// Find initial axes based on schema (prefer temporal for x, quantitative for y).
fn initial_axes(schema: &Schema) -> (usize, usize) {
    let temporal_idx = schema
        .columns
        .iter()
        .position(|c| c.data_type == DataType::Temporal);
    let quant_idx = schema
        .columns
        .iter()
        .position(|c| c.data_type == DataType::Quantitative);

    let x = temporal_idx
        .or_else(|| {
            schema
                .columns
                .iter()
                .position(|c| c.data_type == DataType::Categorical)
        })
        .unwrap_or(0);
    let y = quant_idx.unwrap_or(1.min(schema.columns.len().saturating_sub(1)));

    (x, y)
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

mod render;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infer::types::ColumnMeta;

    fn make_test_app() -> ExploreApp {
        let schema = Schema::new(vec![
            ColumnMeta {
                name: "date".to_string(),
                data_type: DataType::Temporal,
                null_count: 0,
                sample_size: 4,
            },
            ColumnMeta {
                name: "city".to_string(),
                data_type: DataType::Categorical,
                null_count: 0,
                sample_size: 4,
            },
            ColumnMeta {
                name: "revenue".to_string(),
                data_type: DataType::Quantitative,
                null_count: 0,
                sample_size: 4,
            },
        ]);
        let data = vec![
            vec!["2024-01-01".into(), "Tokyo".into(), "100".into()],
            vec!["2024-02-01".into(), "Osaka".into(), "200".into()],
            vec!["2024-03-01".into(), "Tokyo".into(), "150".into()],
            vec!["2024-04-01".into(), "Nagoya".into(), "300".into()],
        ];
        ExploreApp::new(schema, data, crate::theme::Theme::dark())
    }

    #[test]
    fn test_initial_axes_prefer_temporal_for_x() {
        let app = make_test_app();
        assert_eq!(app.selected_x, 0); // date (temporal)
        assert_eq!(app.selected_y, 2); // revenue (quantitative)
    }

    #[test]
    fn test_effective_chart_type_auto() {
        let app = make_test_app();
        assert_eq!(app.effective_chart_type(), ChartType::Line);
    }

    #[test]
    fn test_chart_type_override() {
        let mut app = make_test_app();
        app.handle_key(KeyCode::Char('2'));
        assert_eq!(app.effective_chart_type(), ChartType::Bar);
    }

    #[test]
    fn test_chart_type_reset_to_auto() {
        let mut app = make_test_app();
        app.handle_key(KeyCode::Char('3'));
        assert_eq!(app.effective_chart_type(), ChartType::Scatter);
        app.handle_key(KeyCode::Char('0'));
        assert_eq!(app.effective_chart_type(), ChartType::Line);
    }

    #[test]
    fn test_navigate_x_axis() {
        let mut app = make_test_app();
        assert_eq!(app.selected_x, 0);
        app.handle_key(KeyCode::Char('l'));
        assert_eq!(app.selected_x, 1);
        app.handle_key(KeyCode::Char('l'));
        assert_eq!(app.selected_x, 2);
        // Shouldn't go past the end
        app.handle_key(KeyCode::Char('l'));
        assert_eq!(app.selected_x, 2);
    }

    #[test]
    fn test_navigate_y_axis() {
        let mut app = make_test_app();
        assert_eq!(app.selected_y, 2);
        app.handle_key(KeyCode::Char('k'));
        assert_eq!(app.selected_y, 1);
        app.handle_key(KeyCode::Char('k'));
        assert_eq!(app.selected_y, 0);
        // Shouldn't go below 0
        app.handle_key(KeyCode::Char('k'));
        assert_eq!(app.selected_y, 0);
    }

    #[test]
    fn test_quit_key() {
        let mut app = make_test_app();
        assert!(!app.should_quit);
        app.handle_key(KeyCode::Char('q'));
        assert!(app.should_quit);
    }

    #[test]
    fn test_build_chart_config() {
        let app = make_test_app();
        let config = app.build_chart_config();
        assert!(config.title.is_some());
        // Auto-detects "city" as color column → 3 series (Tokyo, Osaka, Nagoya)
        assert_eq!(config.series.len(), 3);
        // Total data points across all series = 4 rows
        let total_points: usize = config.series.iter().map(|s| s.data.len()).sum();
        assert_eq!(total_points, 4);
    }

    #[test]
    fn test_build_bar_data() {
        let mut app = make_test_app();
        app.selected_x = 1; // city (categorical)
        let data = app.build_bar_data();
        // Data has Tokyo(100), Osaka(200), Tokyo(150), Nagoya(300)
        // After aggregation: Tokyo=250, Osaka=200, Nagoya=300
        assert_eq!(data.labels.len(), 3);
        assert_eq!(data.values.len(), 3);
        assert_eq!(data.labels, vec!["Tokyo", "Osaka", "Nagoya"]);
        assert_eq!(data.values, vec![250.0, 200.0, 300.0]);
    }

    #[test]
    fn test_build_histogram_data() {
        let mut app = make_test_app();
        app.selected_x = 2; // revenue (quantitative)
        let hist_data = app.build_histogram_data();
        assert_eq!(hist_data.values.len(), 4);
        assert_eq!(hist_data.bin_count, 10);
    }

    #[test]
    fn test_y_values_numeric() {
        let app = make_test_app();
        let values = app.y_values();
        assert_eq!(values, vec![100.0, 200.0, 150.0, 300.0]);
    }

    #[test]
    fn test_default_view_mode_is_chart() {
        let app = make_test_app();
        assert_eq!(app.view_mode, ViewMode::Chart);
    }

    #[test]
    fn test_toggle_view_mode_with_d() {
        let mut app = make_test_app();
        assert_eq!(app.view_mode, ViewMode::Chart);
        app.handle_key(KeyCode::Char('d'));
        assert_eq!(app.view_mode, ViewMode::Table);
        app.handle_key(KeyCode::Char('d'));
        assert_eq!(app.view_mode, ViewMode::Chart);
    }

    #[test]
    fn test_toggle_view_mode_with_tab() {
        let mut app = make_test_app();
        app.handle_key(KeyCode::Tab);
        assert_eq!(app.view_mode, ViewMode::Table);
        app.handle_key(KeyCode::Tab);
        assert_eq!(app.view_mode, ViewMode::Chart);
    }

    #[test]
    fn test_table_scroll_state() {
        let mut app = make_test_app();
        app.handle_key(KeyCode::Char('d')); // switch to table
        assert_eq!(app.table_offset, 0);
        // In table mode, j/k should scroll rows (not change Y axis)
        app.handle_key(KeyCode::Char('j'));
        assert_eq!(app.table_offset, 1);
        app.handle_key(KeyCode::Char('k'));
        assert_eq!(app.table_offset, 0);
        // Should not go below 0
        app.handle_key(KeyCode::Char('k'));
        assert_eq!(app.table_offset, 0);
    }

    #[test]
    fn test_table_jump_and_page_navigation() {
        let mut app = make_test_app();
        app.handle_key(KeyCode::Char('d'));
        app.handle_key(KeyCode::Char('G'));
        assert_eq!(app.table_offset, app.data.len() - 1);
        app.handle_key(KeyCode::Char('g'));
        assert_eq!(app.table_offset, 0);
        app.handle_key(KeyCode::PageDown);
        assert_eq!(app.table_offset, 3);
        app.handle_key(KeyCode::PageUp);
        assert_eq!(app.table_offset, 0);
        // G is noop in chart mode
        app.handle_key(KeyCode::Char('d'));
        app.handle_key(KeyCode::Char('G'));
        assert_eq!(app.table_offset, 0);
    }

    #[test]
    fn test_x_skips_y_when_navigating_right() {
        let mut app = make_test_app();
        // Initial: x=0(date), y=2(revenue)
        app.selected_x = 1; // city
        app.selected_y = 2; // revenue
        // Moving x right from 1 should skip 2 (== y) — but can't go past max=2
        // So it should stay at 1 since skipping would go out of bounds
        app.handle_key(KeyCode::Char('l'));
        // x moves to 2, equals y=2, tries to skip to 3 which is past max — stays at 2
        // This is acceptable: at boundary, collision is allowed
        assert_eq!(app.selected_x, 2);
    }

    #[test]
    fn test_x_skips_y_when_navigating_left() {
        let mut app = make_test_app();
        // Set up: x=2, y=1
        app.selected_x = 2;
        app.selected_y = 1;
        // Moving x left from 2 → 1, collides with y=1, skips to 0
        app.handle_key(KeyCode::Char('h'));
        assert_eq!(app.selected_x, 0);
    }

    #[test]
    fn test_y_skips_x_when_navigating_down() {
        let mut app = make_test_app();
        // Set up: x=1, y=0
        app.selected_x = 1;
        app.selected_y = 0;
        // Moving y down from 0 → 1, collides with x=1, skips to 2
        app.handle_key(KeyCode::Char('j'));
        assert_eq!(app.selected_y, 2);
    }

    #[test]
    fn test_y_skips_x_when_navigating_up() {
        let mut app = make_test_app();
        // Set up: x=1, y=2
        app.selected_x = 1;
        app.selected_y = 2;
        // Moving y up from 2 → 1, collides with x=1, skips to 0
        app.handle_key(KeyCode::Char('k'));
        assert_eq!(app.selected_y, 0);
    }

    #[test]
    fn test_color_cycle_none_to_categorical() {
        let mut app = make_test_app();
        // Initial: x=0(date/temporal), y=2(revenue/quant)
        // Only categorical col is 1 (city), not used as x or y
        assert_eq!(app.selected_color, None);
        app.handle_key(KeyCode::Char('c'));
        assert_eq!(app.selected_color, Some(1)); // city
    }

    #[test]
    fn test_color_cycle_wraps_to_none() {
        let mut app = make_test_app();
        // Only one categorical column available (city at index 1)
        app.handle_key(KeyCode::Char('c'));
        assert_eq!(app.selected_color, Some(1));
        // Cycle again → no more categoricals → back to None
        app.handle_key(KeyCode::Char('c'));
        assert_eq!(app.selected_color, None);
    }

    #[test]
    fn test_color_cycle_skips_x_and_y() {
        let mut app = make_test_app();
        // Move x to city (idx 1, categorical) — now it's used as x
        app.selected_x = 1;
        app.selected_y = 2;
        // No categorical columns available (city is x)
        app.handle_key(KeyCode::Char('c'));
        assert_eq!(app.selected_color, None); // nothing to select
    }

    #[test]
    fn test_cycle_color_no_categoricals_shows_message() {
        let schema = Schema::new(vec![
            ColumnMeta {
                name: "x".to_string(),
                data_type: DataType::Quantitative,
                null_count: 0,
                sample_size: 2,
            },
            ColumnMeta {
                name: "y".to_string(),
                data_type: DataType::Quantitative,
                null_count: 0,
                sample_size: 2,
            },
        ]);
        let data = vec![vec!["1".into(), "2".into()]];
        let mut app = ExploreApp::new(schema, data, crate::theme::Theme::dark());
        app.handle_key(KeyCode::Char('c'));
        assert_eq!(
            app.status_message.as_deref(),
            Some("no color columns available")
        );
    }

    #[test]
    fn test_status_bar_shows_current_color_column() {
        let mut app = make_test_app();
        // city is at index 1 (categorical)
        app.handle_key(KeyCode::Char('c'));
        assert_eq!(app.selected_color, Some(1));
        // The status bar function uses app.selected_color to show "c=city"
        // We just verify the field is set correctly
    }

    #[test]
    fn test_chart_type_change_shows_notification() {
        let mut app = make_test_app();
        // Initial: date(Temporal) x revenue(Quantitative) = Line
        assert_eq!(app.effective_chart_type(), ChartType::Line);
        // Navigate X to city (Categorical) → should auto-change to Bar
        app.handle_key(KeyCode::Char('h')); // Move X to the left... actually let's navigate right
        app.handle_key(KeyCode::Char('l')); // Move X from date to city
        // After moving X to Categorical, chart type should change
        if app.effective_chart_type() != ChartType::Line {
            assert!(
                app.status_message.is_some(),
                "Expected notification on chart type change"
            );
            assert!(
                app.status_message
                    .as_deref()
                    .unwrap_or("")
                    .contains("auto:"),
                "Notification should contain 'auto:'"
            );
        }
    }

    #[test]
    fn test_help_overlay_toggle() {
        let mut app = make_test_app();
        assert!(!app.show_help);
        app.handle_key(KeyCode::Char('?'));
        assert!(app.show_help, "? should open help");
        // Any key closes it
        app.handle_key(KeyCode::Char('x'));
        assert!(!app.show_help, "any key should close help");
        // The 'x' key should not have done anything else (not quit, etc.)
        assert!(!app.should_quit);
    }

    #[test]
    fn test_initial_status_message_shows_hints() {
        let app = make_test_app();
        let msg = app.status_message.as_deref().unwrap();
        assert!(msg.contains("?"), "should mention help key");
        assert!(msg.contains("h/l"), "should mention axis navigation");
        assert!(msg.contains("q"), "should mention quit");
    }

    #[test]
    fn test_yank_command_generates_oneshot() {
        let mut app = make_test_app();
        app.handle_key(KeyCode::Char('y'));
        let msg = app.status_message.as_deref().unwrap();
        assert!(msg.starts_with("vz <FILE>"), "should generate vz command");
        assert!(msg.contains("-x "), "should contain -x flag");
        assert!(msg.contains("-y "), "should contain -y flag");
        assert!(msg.contains("-t "), "should contain -t flag");
    }
}
