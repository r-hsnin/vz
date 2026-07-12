//! Chart data loading for presentation mode: resolve paths, load data, build ChartData.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::chart::data_builder;
use crate::chart::selector::ChartType;
use crate::cli::AggFunction;

use super::ChartBlock;

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

/// Load chart data from a chart block definition and base directory.
pub fn load_chart_data(
    block: &ChartBlock,
    base_dir: &Path,
    theme: &crate::theme::Theme,
) -> Result<crate::render::ChartData> {
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
    build_chart_data_for_type(chart_type, block, rows, &axes, theme)
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
    theme: &crate::theme::Theme,
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
            let (mut data, _) = data_builder::aggregate_bar(
                rows,
                cols.x_idx,
                cols.y_idx,
                block.title.clone(),
                cols.y_label.clone(),
                AggFunction::Sum,
            );
            data.series_colors = theme.series_colors.clone();
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
            let mut config = data_builder::build_chart_config(
                rows,
                cols.x_idx,
                cols.y_idx,
                cols.color_idx,
                cols.x_label.clone(),
                cols.y_label.clone(),
                block.title.clone(),
            );
            config.series_colors = theme.series_colors.clone();
            config.axis_color = Some(theme.axis_color);
            config.label_color = Some(theme.label_color);
            if chart_type == ChartType::Scatter {
                Ok(ChartData::Scatter(config))
            } else {
                Ok(ChartData::Line(config))
            }
        }
    }
}
