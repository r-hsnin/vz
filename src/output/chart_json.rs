//! Chart data JSON output: structured JSON including chart_data field.

use std::path::Path;

use anyhow::Result;
use serde_json::json;

use crate::chart::data_builder;
use crate::chart::selector::ChartType;
use crate::cli::{AggFunction, SortOrder};
use crate::infer::types::Schema;
use crate::loader::LoadedData;
use crate::oneshot;
use crate::render;

use super::build_info_output;

/// Parameters for chart JSON output (avoids passing full Cli struct).
pub struct ChartJsonParams {
    pub chart_type: ChartType,
    pub sort: Option<SortOrder>,
    pub agg: AggFunction,
    pub limit: Option<usize>,
    pub extra_y_columns: Vec<(String, Option<String>)>,
    pub color_column: Option<String>,
}

/// Print chart data as JSON (metadata + chart_data field).
pub fn print_chart_json(
    file: &Path,
    data: &LoadedData,
    schema: &Schema,
    recommendation: &crate::chart::ChartRecommendation,
    headers: &[String],
    rows: &[Vec<String>],
    params: &ChartJsonParams,
) -> Result<()> {
    let output = build_info_output(
        &file.display().to_string(),
        data,
        schema,
        Some(recommendation),
    );

    let x_idx = data_builder::column_index(headers, &recommendation.x_column).unwrap_or(0);
    let y_idx = recommendation
        .y_column
        .as_ref()
        .and_then(|y| data_builder::column_index(headers, y))
        .unwrap_or(x_idx);

    let chart_data = build_chart_data(headers, rows, x_idx, y_idx, params);

    let mut output_value = serde_json::to_value(&output)?;
    if let serde_json::Value::Object(ref mut map) = output_value {
        map.insert("chart_data".to_string(), chart_data);
    }

    println!("{}", serde_json::to_string_pretty(&output_value)?);
    Ok(())
}

/// Build the chart_data JSON value based on chart type.
fn build_chart_data(
    headers: &[String],
    rows: &[Vec<String>],
    x_idx: usize,
    y_idx: usize,
    params: &ChartJsonParams,
) -> serde_json::Value {
    match params.chart_type {
        ChartType::Bar => build_bar_json(rows, x_idx, y_idx, params),
        ChartType::Histogram => build_histogram_json(rows, y_idx),
        _ => build_series_json(headers, rows, x_idx, y_idx, params),
    }
}

/// Build bar chart JSON with aggregation, sort, and limit.
fn build_bar_json(
    rows: &[Vec<String>],
    x_idx: usize,
    y_idx: usize,
    params: &ChartJsonParams,
) -> serde_json::Value {
    let (mut bar_data, _) =
        data_builder::aggregate_bar(rows, x_idx, y_idx, None, String::new(), params.agg);
    oneshot::builders::sort_bar_data(&mut bar_data, params.sort);
    if let Some(n) = params.limit {
        bar_data.labels.truncate(n);
        bar_data.values.truncate(n);
    }
    json!({ "type": "bar", "categories": bar_data.labels, "values": bar_data.values })
}

/// Build histogram JSON with bin ranges and counts.
fn build_histogram_json(rows: &[Vec<String>], x_idx: usize) -> serde_json::Value {
    let hist_data = data_builder::build_histogram(rows, x_idx, None, String::new(), None);
    let bins = render::compute_bins(&hist_data.values, hist_data.bin_count);
    let bin_data: Vec<serde_json::Value> = bins
        .iter()
        .map(|(s, e, c)| json!({"range": format!("{:.0}-{:.0}", s, e), "count": c}))
        .collect();
    json!({ "type": "histogram", "bins": bin_data })
}

/// Build line/scatter series JSON, with optional color-group splitting.
fn build_series_json(
    headers: &[String],
    rows: &[Vec<String>],
    x_idx: usize,
    y_idx: usize,
    params: &ChartJsonParams,
) -> serde_json::Value {
    // If color_column is set, group rows by color value → one series per group
    if let Some(ref color_col) = params.color_column
        && let Some(ci) = headers.iter().position(|h| h == color_col)
    {
        return build_grouped_series_json(rows, x_idx, y_idx, ci, params);
    }

    // No color grouping: single series (+ extra Y columns)
    let extra_y: Vec<usize> = params
        .extra_y_columns
        .iter()
        .filter_map(|(name, _)| headers.iter().position(|h| h == name))
        .collect();

    let mut series: Vec<serde_json::Value> = Vec::new();
    let y_name = headers.get(y_idx).cloned().unwrap_or_default();
    let points: Vec<serde_json::Value> = rows
        .iter()
        .filter_map(|r| {
            let x = r.get(x_idx)?.clone();
            let y: f64 = r.get(y_idx)?.replace(',', "").parse().ok()?;
            Some(json!({"x": x, "y": y}))
        })
        .collect();
    series.push(json!({"name": y_name, "data": points}));

    for &ey in &extra_y {
        let name = headers.get(ey).cloned().unwrap_or_default();
        let pts: Vec<serde_json::Value> = rows
            .iter()
            .filter_map(|r| {
                let x = r.get(x_idx)?.clone();
                let y: f64 = r.get(ey)?.replace(',', "").parse().ok()?;
                Some(json!({"x": x, "y": y}))
            })
            .collect();
        series.push(json!({"name": name, "data": pts}));
    }

    json!({ "type": params.chart_type.to_string().to_lowercase(), "series": series })
}

/// Build series JSON grouped by color column values.
fn build_grouped_series_json(
    rows: &[Vec<String>],
    x_idx: usize,
    y_idx: usize,
    color_idx: usize,
    params: &ChartJsonParams,
) -> serde_json::Value {
    use std::collections::BTreeMap;

    let mut groups: BTreeMap<String, Vec<serde_json::Value>> = BTreeMap::new();
    for row in rows {
        let group = row.get(color_idx).cloned().unwrap_or_default();
        let point = (|| {
            let x = row.get(x_idx)?.clone();
            let y: f64 = row.get(y_idx)?.replace(',', "").parse().ok()?;
            Some(json!({"x": x, "y": y}))
        })();
        if let Some(pt) = point {
            groups.entry(group).or_default().push(pt);
        }
    }

    let series: Vec<serde_json::Value> = groups
        .into_iter()
        .map(|(name, data)| json!({"name": name, "data": data}))
        .collect();

    json!({ "type": params.chart_type.to_string().to_lowercase(), "series": series })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_params(chart_type: ChartType) -> ChartJsonParams {
        ChartJsonParams {
            chart_type,
            sort: None,
            agg: AggFunction::Sum,
            limit: None,
            extra_y_columns: vec![],
            color_column: None,
        }
    }

    #[test]
    fn test_build_bar_json_aggregates() {
        let rows = vec![
            vec!["Tokyo".into(), "1000".into()],
            vec!["Tokyo".into(), "2000".into()],
            vec!["Osaka".into(), "1500".into()],
        ];
        let params = make_params(ChartType::Bar);
        let result = build_bar_json(&rows, 0, 1, &params);
        assert_eq!(result["type"], "bar");
        let categories = result["categories"].as_array().unwrap();
        let values = result["values"].as_array().unwrap();
        assert_eq!(categories.len(), 2);
        assert_eq!(values.len(), 2);
        // Tokyo=3000, Osaka=1500
        assert!(categories.contains(&json!("Tokyo")));
        assert!(categories.contains(&json!("Osaka")));
    }

    #[test]
    fn test_build_bar_json_with_sort_and_limit() {
        let rows = vec![
            vec!["A".into(), "100".into()],
            vec!["B".into(), "300".into()],
            vec!["C".into(), "200".into()],
        ];
        let params = ChartJsonParams {
            chart_type: ChartType::Bar,
            sort: Some(SortOrder::Desc),
            agg: AggFunction::Sum,
            limit: Some(2),
            extra_y_columns: vec![],
            color_column: None,
        };
        let result = build_bar_json(&rows, 0, 1, &params);
        let categories = result["categories"].as_array().unwrap();
        let values = result["values"].as_array().unwrap();
        assert_eq!(categories.len(), 2);
        // Sorted desc: B(300), C(200) — A(100) truncated
        assert_eq!(categories[0], "B");
        assert_eq!(values[0], 300.0);
    }

    #[test]
    fn test_build_histogram_json() {
        let rows = vec![
            vec!["10".into()],
            vec!["20".into()],
            vec!["30".into()],
            vec!["40".into()],
            vec!["50".into()],
        ];
        let result = build_histogram_json(&rows, 0);
        assert_eq!(result["type"], "histogram");
        let bins = result["bins"].as_array().unwrap();
        assert!(!bins.is_empty());
        // Each bin should have "range" and "count"
        assert!(bins[0]["range"].is_string());
        assert!(bins[0]["count"].is_number());
    }

    #[test]
    fn test_build_series_json_single() {
        let headers = vec!["date".into(), "value".into()];
        let rows = vec![
            vec!["2024-01".into(), "100".into()],
            vec!["2024-02".into(), "200".into()],
        ];
        let params = make_params(ChartType::Line);
        let result = build_series_json(&headers, &rows, 0, 1, &params);
        assert_eq!(result["type"], "line");
        let series = result["series"].as_array().unwrap();
        assert_eq!(series.len(), 1);
        assert_eq!(series[0]["name"], "value");
        let data = series[0]["data"].as_array().unwrap();
        assert_eq!(data.len(), 2);
        assert_eq!(data[0]["x"], "2024-01");
        assert_eq!(data[0]["y"], 100.0);
    }

    #[test]
    fn test_build_series_json_extra_y() {
        let headers = vec!["date".into(), "revenue".into(), "profit".into()];
        let rows = vec![
            vec!["2024-01".into(), "1000".into(), "200".into()],
            vec!["2024-02".into(), "1500".into(), "300".into()],
        ];
        let params = ChartJsonParams {
            chart_type: ChartType::Line,
            sort: None,
            agg: AggFunction::Sum,
            limit: None,
            extra_y_columns: vec![("profit".into(), None)],
            color_column: None,
        };
        let result = build_series_json(&headers, &rows, 0, 1, &params);
        let series = result["series"].as_array().unwrap();
        assert_eq!(series.len(), 2);
        assert_eq!(series[0]["name"], "revenue");
        assert_eq!(series[1]["name"], "profit");
        assert_eq!(series[1]["data"].as_array().unwrap()[0]["y"], 200.0);
    }

    #[test]
    fn test_build_grouped_series_json() {
        let rows = vec![
            vec!["2024-01".into(), "100".into(), "Tokyo".into()],
            vec!["2024-02".into(), "200".into(), "Osaka".into()],
            vec!["2024-03".into(), "150".into(), "Tokyo".into()],
        ];
        let params = make_params(ChartType::Line);
        let result = build_grouped_series_json(&rows, 0, 1, 2, &params);
        assert_eq!(result["type"], "line");
        let series = result["series"].as_array().unwrap();
        assert_eq!(series.len(), 2); // Osaka, Tokyo (BTreeMap = sorted)
        assert_eq!(series[0]["name"], "Osaka");
        assert_eq!(series[0]["data"].as_array().unwrap().len(), 1);
        assert_eq!(series[1]["name"], "Tokyo");
        assert_eq!(series[1]["data"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_build_series_json_with_color_column() {
        let headers = vec!["date".into(), "value".into(), "city".into()];
        let rows = vec![
            vec!["2024-01".into(), "100".into(), "A".into()],
            vec!["2024-02".into(), "200".into(), "B".into()],
        ];
        let params = ChartJsonParams {
            chart_type: ChartType::Scatter,
            sort: None,
            agg: AggFunction::Sum,
            limit: None,
            extra_y_columns: vec![],
            color_column: Some("city".into()),
        };
        let result = build_series_json(&headers, &rows, 0, 1, &params);
        assert_eq!(result["type"], "scatter");
        let series = result["series"].as_array().unwrap();
        assert_eq!(series.len(), 2);
        assert_eq!(series[0]["name"], "A");
        assert_eq!(series[1]["name"], "B");
    }

    #[test]
    fn test_build_chart_data_dispatches_correctly() {
        let headers = vec!["x".into(), "y".into()];
        let rows = vec![vec!["a".into(), "1".into()], vec!["b".into(), "2".into()]];

        // Bar
        let bar_params = make_params(ChartType::Bar);
        let bar_result = build_chart_data(&headers, &rows, 0, 1, &bar_params);
        assert_eq!(bar_result["type"], "bar");

        // Line
        let line_params = make_params(ChartType::Line);
        let line_result = build_chart_data(&headers, &rows, 0, 1, &line_params);
        assert_eq!(line_result["type"], "line");
    }
}
