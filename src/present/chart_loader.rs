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
            "Chart source not found: {} (tried: {})",
            block.source,
            path.display()
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
            let agg_fn = block.agg.unwrap_or(AggFunction::Sum);
            let (mut data, _) = data_builder::aggregate_bar(
                rows,
                cols.x_idx,
                cols.y_idx,
                block.title.clone(),
                cols.y_label.clone(),
                agg_fn,
            );
            let sort = block
                .top
                .map(|_| crate::cli::SortOrder::Desc)
                .or(block.sort);
            crate::oneshot::builders::sort_bar_data(&mut data, sort);
            crate::oneshot::builders::truncate_bar_data(&mut data, block.top);
            data.series_colors = theme.series_colors.clone();
            data.axis_color = Some(theme.axis_color);
            Ok(ChartData::Bar(data))
        }
        ChartType::Histogram => {
            let mut data = data_builder::build_histogram(
                rows,
                cols.x_idx,
                block.title.clone(),
                cols.x_label.clone(),
                block.bins,
            );
            data.axis_color = Some(theme.axis_color);
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
            config.apply_theme(theme);
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
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_resolve_chart_source_path_absolute() {
        let base_dir = Path::new("/some/dir");
        let result = resolve_chart_source_path("/absolute/path/data.csv", base_dir);
        assert_eq!(result, PathBuf::from("/absolute/path/data.csv"));
    }

    #[test]
    fn test_resolve_chart_source_path_relative_existing() {
        let tmp = TempDir::new().unwrap();
        let csv_path = tmp.path().join("data.csv");
        fs::write(&csv_path, "a,b\n1,2\n").unwrap();

        let result = resolve_chart_source_path("data.csv", tmp.path());
        assert_eq!(result, csv_path);
    }

    #[test]
    fn test_resolve_chart_source_path_relative_nonexistent() {
        let tmp = TempDir::new().unwrap();
        // File does not exist in base_dir — falls back to raw relative path
        let result = resolve_chart_source_path("missing.csv", tmp.path());
        assert_eq!(result, PathBuf::from("missing.csv"));
    }

    #[test]
    fn test_infer_chart_type_temporal_x() {
        let headers = vec!["date".to_string(), "value".to_string()];
        let rows = vec![
            vec!["2024-01-01".to_string(), "100".to_string()],
            vec!["2024-02-01".to_string(), "200".to_string()],
            vec!["2024-03-01".to_string(), "150".to_string()],
        ];
        let block = ChartBlock {
            source: String::new(),
            chart_type: None,
            x_col: None,
            y_col: None,
            color_col: None,
            title: None,
            filter: vec![],
            sort: None,
            agg: None,
            top: None,
            bins: None,
            height: None,
        };
        let ct = infer_chart_type_from_data(&headers, &rows, &block);
        assert_eq!(ct, ChartType::Line);
    }

    #[test]
    fn test_infer_chart_type_categorical_x() {
        let headers = vec!["city".to_string(), "revenue".to_string()];
        let rows = vec![
            vec!["Tokyo".to_string(), "1000".to_string()],
            vec!["Osaka".to_string(), "1500".to_string()],
            vec!["Nagoya".to_string(), "800".to_string()],
        ];
        let block = ChartBlock {
            source: String::new(),
            chart_type: None,
            x_col: None,
            y_col: None,
            color_col: None,
            title: None,
            filter: vec![],
            sort: None,
            agg: None,
            top: None,
            bins: None,
            height: None,
        };
        let ct = infer_chart_type_from_data(&headers, &rows, &block);
        assert_eq!(ct, ChartType::Bar);
    }

    #[test]
    fn test_infer_chart_type_quantitative_both() {
        let headers = vec!["height".to_string(), "weight".to_string()];
        let rows = vec![
            vec!["170".to_string(), "65".to_string()],
            vec!["180".to_string(), "75".to_string()],
            vec!["165".to_string(), "58".to_string()],
        ];
        let block = ChartBlock {
            source: String::new(),
            chart_type: None,
            x_col: None,
            y_col: None,
            color_col: None,
            title: None,
            filter: vec![],
            sort: None,
            agg: None,
            top: None,
            bins: None,
            height: None,
        };
        let ct = infer_chart_type_from_data(&headers, &rows, &block);
        assert_eq!(ct, ChartType::Scatter);
    }

    #[test]
    fn test_infer_chart_type_with_x_hint() {
        let headers = vec![
            "date".to_string(),
            "city".to_string(),
            "revenue".to_string(),
        ];
        let rows = vec![
            vec![
                "2024-01-01".to_string(),
                "Tokyo".to_string(),
                "1000".to_string(),
            ],
            vec![
                "2024-02-01".to_string(),
                "Osaka".to_string(),
                "1500".to_string(),
            ],
        ];
        let block = ChartBlock {
            source: String::new(),
            chart_type: None,
            x_col: Some("city".to_string()),
            y_col: Some("revenue".to_string()),
            color_col: None,
            title: None,
            filter: vec![],
            sort: None,
            agg: None,
            top: None,
            bins: None,
            height: None,
        };
        let ct = infer_chart_type_from_data(&headers, &rows, &block);
        assert_eq!(ct, ChartType::Bar);
    }
}
