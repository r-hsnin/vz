//! Chart data loading for presentation mode: resolve paths, load data, build ChartData.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::chart::data_builder;
use crate::chart::selector::AggFunction;
use crate::chart::selector::ChartType;

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

/// Load diff chart data: compare two files and produce a bar or line chart.
fn load_diff_chart_data(
    block: &ChartBlock,
    diff_source: &str,
    base_dir: &Path,
    theme: &crate::theme::Theme,
) -> Result<crate::render::ChartData> {
    use crate::diff::{compute_diff, compute_diff_temporal, validate_schema};
    use crate::render::ChartData;

    let before_path = resolve_chart_source_path(&block.source, base_dir);
    let after_path = resolve_chart_source_path(diff_source, base_dir);

    let before = crate::loader::load_data(&before_path).with_context(|| {
        format!(
            "Diff source (before) not found: {} (tried: {})",
            block.source,
            before_path.display()
        )
    })?;
    let after = crate::loader::load_data(&after_path).with_context(|| {
        format!(
            "Diff source (after) not found: {} (tried: {})",
            diff_source,
            after_path.display()
        )
    })?;

    validate_schema(&before, &after, &before_path, &after_path)?;

    // Infer schema to resolve x/y columns and detect temporal vs categorical.
    let schema = crate::pipeline::infer_from_data(&before);

    // Resolve X column: explicit from block, or auto-detect first categorical/temporal.
    let x_col = if let Some(ref x) = block.x_col {
        x.clone()
    } else {
        crate::diff::auto_x_column(&schema, &before.headers)
            .unwrap_or_else(|| before.headers.first().cloned().unwrap_or_default())
    };

    // Resolve Y column: explicit from block, or auto-detect first quantitative (not x).
    let y_col = if let Some(ref y) = block.y_col {
        y.clone()
    } else {
        crate::diff::auto_y_column(&schema, &x_col)
            .ok_or_else(|| anyhow::anyhow!("No quantitative column found for Y axis"))?
    };

    // Determine if X is temporal.
    let x_is_temporal = crate::diff::is_temporal_column(&schema, &x_col);

    if x_is_temporal {
        // Temporal diff → 2-series line chart overlay (canonical assembler;
        // title defaults to the Diff pair, theme comes from the slide).
        let ts = compute_diff_temporal(&before, &after, &x_col, &y_col)?;
        let mut config = data_builder::build_diff_line_config(
            &ts.before,
            &ts.after,
            &ts.x_labels,
            &ts.x_column,
            &ts.y_column,
            block
                .title
                .clone()
                .or_else(|| Some(format!("Diff: {} vs {}", block.source, diff_source))),
        );
        config.axis_color = Some(theme.axis_color);
        config.label_color = Some(theme.label_color);
        Ok(ChartData::Line(config))
    } else {
        // Categorical diff → bar chart with after values and annotated
        // labels (canonical assembler; sort/limit + theme stay at the edge).
        let diff = compute_diff(&before, &after, &x_col, &y_col)?;
        // `top:` implies desc sort (same funnel as `Cli::effective_sort`).
        let sort = block
            .sort
            .or(block.top.map(|_| crate::chart::selector::SortOrder::Desc));
        let tuples: Vec<(String, f64, Option<f64>, f64)> = diff
            .entries
            .iter()
            .map(|e| (e.label.clone(), e.after, e.pct_change, e.delta))
            .collect();
        let mut bar_data = data_builder::build_diff_bar_data(
            &tuples,
            sort,
            block.top,
            y_col,
            block
                .title
                .clone()
                .or_else(|| Some(format!("Diff: {} vs {}", block.source, diff_source))),
        );
        bar_data.series_colors = theme.series_colors.clone();
        bar_data.axis_color = Some(theme.axis_color);
        Ok(ChartData::Bar(bar_data))
    }
}

/// Load chart data from a chart block definition and base directory.
pub fn load_chart_data(
    block: &ChartBlock,
    base_dir: &Path,
    theme: &crate::theme::Theme,
) -> Result<crate::render::ChartData> {
    // If diff mode is specified, branch into diff-specific loading.
    if let Some(ref diff_source) = block.diff {
        return load_diff_chart_data(block, diff_source, base_dir, theme);
    }

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
    )
    .with_context(|| format!("Invalid chart block (source: {})", block.source))?;
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
    let recommendation = crate::chart::select_chart(&schema, x_hint, y_hint);
    if let Ok(ref rec) = recommendation
        && let Some(warning) = crate::chart::selector::fallback_warning(
            &schema,
            &rec.x_column,
            rec.y_column.as_deref(),
            rec.chart_type,
        )
    {
        eprintln!("{warning}");
    }
    recommendation
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
                .sort
                .or(block.top.map(|_| crate::chart::selector::SortOrder::Desc));
            data_builder::sort_bar_data(&mut data, sort);
            data_builder::truncate_bar_data(&mut data, block.top);
            data.series_colors = theme.series_colors.clone();
            data.axis_color = Some(theme.axis_color);
            Ok(ChartData::Bar(data))
        }
        ChartType::Histogram => {
            // Same bin-column choice as oneshot/JSON (canonical).
            let col_idx = data_builder::histogram_column(rows, cols.x_idx, cols.y_idx);
            let label = if col_idx == cols.y_idx {
                cols.y_label.clone()
            } else {
                cols.x_label.clone()
            };
            let mut data = data_builder::build_histogram(
                rows,
                col_idx,
                block.title.clone(),
                label,
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
            diff: None,
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
            diff: None,
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
            diff: None,
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
            diff: None,
        };
        let ct = infer_chart_type_from_data(&headers, &rows, &block);
        assert_eq!(ct, ChartType::Bar);
    }
}
