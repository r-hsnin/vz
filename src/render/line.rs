use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    symbols::Marker,
    text::{Line as TextLine, Span},
    widgets::{Axis as RatatuiAxis, Block, Borders, Chart, Dataset, GraphType, Widget},
};

use super::{ChartConfig, SERIES_COLORS};

/// Rendering mode for XY chart widgets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XYMode {
    /// Line chart: multi-point uses Braille+Line, single-point uses Dot+Scatter.
    Line,
    /// Scatter plot: multi-point uses Braille+Scatter, single-point uses Dot+Scatter.
    Scatter,
}

/// Specification for a dataset's visual style.
#[derive(Debug, Clone, PartialEq)]
struct DatasetSpec {
    marker: Marker,
    graph_type: GraphType,
}

/// Determine marker and graph type based on mode and series length.
fn dataset_spec(mode: XYMode, series_len: usize) -> DatasetSpec {
    match mode {
        XYMode::Scatter => {
            if series_len <= 1 {
                DatasetSpec {
                    marker: Marker::Dot,
                    graph_type: GraphType::Scatter,
                }
            } else {
                DatasetSpec {
                    marker: Marker::Braille,
                    graph_type: GraphType::Scatter,
                }
            }
        }
        XYMode::Line => {
            if series_len <= 1 {
                DatasetSpec {
                    marker: Marker::Dot,
                    graph_type: GraphType::Scatter,
                }
            } else {
                DatasetSpec {
                    marker: Marker::Braille,
                    graph_type: GraphType::Line,
                }
            }
        }
    }
}

/// Unified XY chart widget (line or scatter) wrapping ratatui's Chart.
pub struct XYChart<'a> {
    config: &'a ChartConfig,
    mode: XYMode,
}

impl<'a> XYChart<'a> {
    pub fn new(config: &'a ChartConfig, mode: XYMode) -> Self {
        Self { config, mode }
    }

    /// Get the color for series at index, using theme colors or fallback.
    fn color_at(&self, index: usize) -> Color {
        if self.config.series_colors.is_empty() {
            SERIES_COLORS[index % SERIES_COLORS.len()]
        } else {
            self.config.series_colors[index % self.config.series_colors.len()]
        }
    }
}

impl<'a> Widget for XYChart<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let datasets: Vec<Dataset> = self
            .config
            .series
            .iter()
            .enumerate()
            .map(|(i, series)| {
                let spec = dataset_spec(self.mode, series.data.len());
                Dataset::default()
                    .name(series.name.as_str())
                    .marker(spec.marker)
                    .graph_type(spec.graph_type)
                    .style(Style::default().fg(self.color_at(i)))
                    .data(&series.data)
            })
            .collect();

        let default_title = match self.mode {
            XYMode::Line => "Line Chart",
            XYMode::Scatter => "Scatter Plot",
        };
        let title = self
            .config
            .title
            .as_deref()
            .unwrap_or(default_title)
            .to_string();

        let axis_style = Style::default().fg(self.config.axis_color.unwrap_or(Color::DarkGray));

        let x_axis = RatatuiAxis::default()
            .title(self.config.x_axis.label.as_str())
            .bounds([self.config.x_axis.min, self.config.x_axis.max])
            .labels(
                self.config
                    .x_labels
                    .clone()
                    .unwrap_or_else(|| self.config.x_axis.tick_labels(5)),
            )
            .style(axis_style);

        let y_axis = RatatuiAxis::default()
            .title(self.config.y_axis.label.as_str())
            .bounds([self.config.y_axis.min, self.config.y_axis.max])
            .labels(self.config.y_axis.tick_labels(5))
            .style(axis_style);

        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .title(
                        TextLine::from(Span::styled(
                            title,
                            Style::default().add_modifier(Modifier::BOLD),
                        ))
                        .centered(),
                    )
                    .borders(Borders::ALL),
            )
            .x_axis(x_axis)
            .y_axis(y_axis);

        chart.render(area, buf);

        // Render in-chart legend for multi-series charts
        if self.config.series.len() > 1 {
            render_legend(self.config, area, buf, |idx| self.color_at(idx));
        }
    }
}

/// Render a compact legend overlay in the top-right corner of the chart area.
/// Only called for multi-series charts (line/scatter).
fn render_legend(
    config: &ChartConfig,
    area: Rect,
    buf: &mut Buffer,
    color_fn: impl Fn(usize) -> Color,
) {
    // Need at least space for border + 1 legend entry
    if area.width < 20 || area.height < 5 {
        return;
    }

    let max_name_len = config
        .series
        .iter()
        .map(|s| s.name.len())
        .max()
        .unwrap_or(0)
        .min(16); // Truncate long names to 16 chars

    // Legend width: "█ " (2) + name + 1 padding on each side
    let legend_entry_width = (2 + max_name_len) as u16;
    // Available height inside chart border (top border + title row eaten, bottom border)
    let available_height = area.height.saturating_sub(4);
    let entries_to_show = (config.series.len() as u16).min(available_height);

    if entries_to_show == 0 {
        return;
    }

    // Position: top-right, inside the block border, with 1 char padding from right border
    let legend_x = area.x + area.width.saturating_sub(legend_entry_width + 2);
    let legend_y = area.y + 2; // Below top border + title

    for (i, series) in config
        .series
        .iter()
        .enumerate()
        .take(entries_to_show as usize)
    {
        let y = legend_y + i as u16;
        if y >= area.y + area.height.saturating_sub(2) {
            break;
        }
        let color = color_fn(i);
        buf.set_string(legend_x, y, "█ ", Style::default().fg(color));
        let name: String = series.name.chars().take(max_name_len).collect();
        buf.set_string(legend_x + 2, y, &name, Style::default());
    }
}

/// Line chart widget (backward-compatible wrapper around XYChart).
pub struct LineChart<'a> {
    chart: XYChart<'a>,
}

impl<'a> LineChart<'a> {
    pub fn new(config: &'a ChartConfig) -> Self {
        Self {
            chart: XYChart::new(config, XYMode::Line),
        }
    }
}

impl<'a> Widget for LineChart<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.chart.render(area, buf);
    }
}

/// Scatter plot widget (backward-compatible wrapper around XYChart).
pub struct ScatterPlot<'a> {
    chart: XYChart<'a>,
}

impl<'a> ScatterPlot<'a> {
    pub fn new(config: &'a ChartConfig) -> Self {
        Self {
            chart: XYChart::new(config, XYMode::Scatter),
        }
    }
}

impl<'a> Widget for ScatterPlot<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.chart.render(area, buf);
    }
}

#[cfg(test)]
#[path = "line_tests.rs"]
mod tests;
